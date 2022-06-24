use std::{
  cell::Cell,
  future::Future,
  marker::PhantomData,
  mem::ManuallyDrop,
  panic::{AssertUnwindSafe, catch_unwind},
  pin::Pin,
  task::{Context, Poll, Waker},
  thread::Result
};
use ointers::Ointer;
use futures_micro::pin;
use stackle::switch::*;
pub use stackle::stack;

/// A [`Future`] which executes a closure on its own stack like a stackful coroutine.
///
/// [`Fiber`]s may wait on [`Future`]s via a provided [`Awaiter`] reference.
pub struct Fiber<'a, R, S> {
  /// This is the stack pointer to resume. We steal the sign bit to indicate whether it is done.
  stack_ptr: Ointer<usize,0,true,0>,
  /// This is just here to get dropped when we're done.
  _stack:    S,
  /// Be invariant on R.
  _phantom:  PhantomData<&'a fn() -> R>,
}

// It is safe to send us across threads as long as we don't borrow anything that could go away.
unsafe impl<R, S> Send for Fiber<'static, R, S> {}
// It is always safe to send references to ourself because we require a mut ref to use.
unsafe impl<'a, R, S> Sync for Fiber<'a, R, S> {}

impl<'a, R, S> Drop for Fiber<'a, R, S> {
  fn drop(&mut self) {
    // If the fiber is not marked done, we must unwind the stack
    // We pessimise against this because futures are typically run to completion.
    if unlikely(self.stack_ptr.stolen() == 0) {
      unsafe { switch(self.stack_ptr.as_ptr(), 0) };
    }
  }
}

impl<'a, R, S> Fiber<'a, R, S>
where S: stack::Stack + 'a {
  #[inline(always)]
  pub fn new<F>(fun: F, stack: S) -> Self
  where F: 'a + FnOnce(&Awaiter) -> R {
    let stack_ptr = unsafe {
      Ointer::new(
        link_closure_detached(stack.end(), move |paused, waker| {
          // If waker is 0, it's not a waker, it's a request to unwind.
          if unlikely(waker == 0) {
            // As we're at the top of the stack, the only thing an unwind here would do is drop
            // `fun`'. Returning would also work, but this function may not return.
            drop(fun);
            switch(paused, 0); // the standard response to a panic request.
            unreachable!()             // we aren't here. needed to prevent a compile error.
          }
          // Now we'll create a awaiter with those values and start off the closure.
          let awaiter = Awaiter {
            stack: Cell::new(paused),
            waker: Cell::new(waker as *const _),
          };
          // This delicate mess moves the provided closure into the new closure we're creating. If we
          // don't do this, we'll be rewarded with a segfault at best.
          let ret = {
            let awaiter = &awaiter;             // Permit us to write `move` on the next line.
            let fun = move || fun(awaiter);     // Move the passed closure into the wrapper closure.
            catch_unwind(AssertUnwindSafe(fun)) // Run the closure, catching any unwind panic.
          };
          // 'move' the value back by not dropping it and returning a pointer to it to take.
          let ret = ManuallyDrop::new(ret);
          let ret = &ret as *const ManuallyDrop<Result<R>> as usize;
          // finish by suspending once more, with that pointer.
          switch(awaiter.stack.get(), ret);
          unreachable!()
        })
      ).steal(0)
    };
    Fiber { stack_ptr, _stack: stack, _phantom: PhantomData }
  }
}

impl<'a, R, S> Future for Fiber<'a, R, S> {
  type Output = Result<R>;
  #[inline(always)]
  fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
    // S might not be Unpin, in the case of e.g. being a slice. It
    // doesn't matter because we only hold it so we can drop it later.
    let this = unsafe { Pin::into_inner_unchecked(self) };
    let sp = this.stack_ptr;
    debug_assert!(sp.stolen() == 0);
    let waker = cx.waker() as *const Waker as usize;
    let switch = unsafe { switch(sp.as_ptr(), waker) };
    // The arg field will now be either 0 (Pending) or anything else (ready, a pointer to a Result).
    if likely(switch.arg == 0) {
      this.stack_ptr = unsafe { Ointer::new(switch.stack) }.steal(0);          // 0 = not done.
      Poll::Pending
    } else {
      this.stack_ptr = unsafe { Ointer::new(switch.stack) }.steal(usize::MAX); // 1 = done.
      let ptr = switch.arg as *const Result<R>;
      Poll::Ready(unsafe { ptr.read() })
    }
  }
}

/// Allows a [`Fiber`] to await [`Future`]s.
pub struct Awaiter {
  /// Paused stack pointer for the resumer.
  stack: Cell<*mut usize>,
  /// Reference to the resumer's waker.
  waker: Cell<*const Waker>,
}

impl Awaiter {
  /// Wait on a [`Future`] inside a [`Fiber`].
  #[inline(always)]
  pub fn wait<F: Future>(&self, future: F) -> F::Output {
    pin!(future);                                                     // Pin the future to the stack.
    let mut ctx = Context::from_waker(unsafe { &*self.waker.get() }); // Use resumer's waker.
    loop {                                                            // Do not return until ready.
      match future.as_mut().poll(&mut ctx) {                          // Poll eagerly.
        Poll::Pending => {
          let switch = unsafe { switch(self.stack.get(), 0) };        // Wait to be woken.
          self.stack.set(switch.stack);                               // Update paused stack pointer.
          if unlikely(switch.arg == 0) { panic!() }                   // Unwind if requested.
          self.waker.set(switch.arg as *const Waker);                 // Update waker ref and continue.
        }
        Poll::Ready(val) => return val,
      }
    }
  }
}

// Just optimisation hint stuff below here.

/// A cold function is unlikely to be called. This is the best mechanism we seem to have in stable
/// rust to guide asm generation as to whether a branch is likely to be taken.
#[cold]
#[inline(always)]
fn cold() {}

/// Signifies that a boolean condition is likely to be true when used in a branch condition.
#[inline(always)]
fn likely(cond: bool) -> bool {
    if !cond { cold() }
    cond
}

/// Signifies that a boolean condition is unlikely to be true when used in a branch condition.
#[inline(always)]
fn unlikely(cond: bool) -> bool {
    if cond { cold() }
    cond
}
