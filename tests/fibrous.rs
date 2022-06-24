use fibrous::{*, stack::*};
use wookie::*;
use std::task::Poll;
use core::future::{pending, ready};

struct Reusable<'a>(&'a AllocatorStackConst<8192>);

unsafe impl<'a> Stack for Reusable<'a> {
  #[inline(always)]
  fn end(&self) -> *mut usize { self.0.end() }
}

fn never(a: &Awaiter) {
  loop { a.wait(pending::<()>()); }
}

fn requires_send<S: Send>(_wat: &S) {}
fn requires_sync<S: Sync>(_wat: &S) {}

#[test]
fn static_is_send() {
  let s = unsafe { AllocatorStackConst::<8192>::new() };
  let s = unsafe { &*(&s as *const _) };
  let s = Fiber::new(|_| 42usize, Reusable(s));
  requires_send(&s);
}

#[test]
fn is_sync() {
  let s = unsafe { AllocatorStackConst::<8192>::new() };
  let s = Fiber::new(|_| 42usize, s);
  requires_sync(&s);
}

#[test]
fn poll_ret() {
  let s = unsafe { AllocatorStackConst::<8192>::new() };
  wookie!(s: Fiber::new(|_| 42usize, s));
  match s.poll() {
    Poll::Ready(Ok(42)) => (),
    _ => unreachable!()
  }
  s.stats().assert(0,0,0);
}

#[test]
fn poll_panic() {
  let s = unsafe { AllocatorStackConst::<8192>::new() };
  wookie!(s: Fiber::new(|_| panic!("no"), Reusable(&s)));
  match s.poll() {
    Poll::Ready(Err(_)) => (),
    _ => unreachable!()
  }
  s.stats().assert(0,0,0);
}

#[test]
fn poll_never() {
  let s = unsafe { AllocatorStackConst::<8192>::new() };
  wookie!(s: Fiber::new(never, Reusable(&s)));
  for _ in 0..5 {
    match s.poll() {
      Poll::Pending => (),
      _ => unreachable!()
    }
  }
  s.stats().assert(0,0,0);
}

#[test]
fn poll_always() {
  let s = unsafe { AllocatorStackConst::<8192>::new() };
  wookie!(s: Fiber::new(|b| b.wait(ready(42usize)), Reusable(&s)));
  match s.poll() {
    Poll::Ready(Ok(42usize)) => (),
    _ => unreachable!()
  }
  s.stats().assert(0,0,0);
}

#[test]
fn drop_unused() {
  let s = unsafe { AllocatorStackConst::<8192>::new() };
  Fiber::new(|_| (), s);
}
