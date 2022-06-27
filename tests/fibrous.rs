use fibrous::{*, stack::*};
use wookie::*;
use std::task::Poll;
use core::future::{pending, ready};

fn never(a: &Awaiter) {
  loop { a.wait(pending::<()>()); }
}

fn requires_send<S: Send>(_wat: &S) {}
fn requires_sync<S: Sync>(_wat: &S) {}

#[allow(dead_code)]
fn static_is_send() {
  let s = unsafe { AllocatorStack::new(8192) };
  let s = Fiber::new(|_| 42usize, s);
  requires_send(&s);
}

#[allow(dead_code)]
fn is_sync() {
  let s = unsafe { AllocatorStack::new(8192) };
  let s = Fiber::new(|_| 42usize, s);
  requires_sync(&s);
}

#[test]
fn poll_ret() {
  let s = unsafe { AllocatorStack::new(8192) };
  wookie!(s: Fiber::new(|_| 42usize, s));
  match s.poll() {
    Poll::Ready(Ok(42)) => (),
    _ => unreachable!()
  }
  s.stats().assert(0,0,0);
}

#[test]
fn poll_panic() {
  let s = unsafe { AllocatorStack::new(8192) };
  wookie!(s: Fiber::new(|_| panic!("no"), s));
  match s.poll() {
    Poll::Ready(Err(_)) => (),
    _ => unreachable!()
  }
  s.stats().assert(0,0,0);
}

#[test]
fn poll_never() {
  let s = unsafe { AllocatorStack::new(8192) };
  wookie!(s: Fiber::new(never, s));
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
  let s = unsafe { AllocatorStack::new(8192) };
  wookie!(s: Fiber::new(|b| b.wait(ready(42usize)), s));
  match s.poll() {
    Poll::Ready(Ok(42usize)) => (),
    _ => unreachable!()
  }
  s.stats().assert(0,0,0);
}

#[test]
fn drop_unused() {
  let s = unsafe { AllocatorStack::new(8192) };
  Fiber::new(|_| (), s);
}
