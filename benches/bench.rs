use criterion::*;
use fibrous::{*, stack::*};
use std::future::pending;
use std::panic;
use wookie::dummy;

struct Reusable<'a, S>(&'a S);

// This is going to help us get more accurate measurements.
// Absolutely do *not* do this in a real program, it's a very dumb idea.
unsafe impl<'a, S: Stack> Stack for Reusable<'a, S> {
  fn end(&self) -> *mut usize { self.0.end() }
}

fn closure(awaiter: &Awaiter) {
  loop { awaiter.wait(pending::<()>()); }
}

fn stack_creation(c: &mut Criterion) {
  let mut group = c.benchmark_group("stack_creation");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "allocator",
    |b| { b.iter(|| black_box(unsafe { AllocatorStack::new(8192) })); }
  );
  group.bench_function(
    "safe",
    |b| {
      let p = PageSize::get().unwrap();
      b.iter(|| black_box(SafeStack::new(8192, p)));
    }
  );
  group.bench_function(
    "paranoid",
    |b| {
      let p = PageSize::get().unwrap();
      b.iter(|| black_box(ParanoidStack::new(8192, p)));
    }
  );
}

fn linking(c: &mut Criterion) {
  // These should all take basically the same time, but they will bounce about a bit.
  panic::set_hook(Box::new(|_| ())); // eliminate noise
  let mut group = c.benchmark_group("linking_closure");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "allocator",
    |b| {
      unsafe {
        let s = AllocatorStack::new(8192);
        b.iter(|| {
          let r = Reusable(&s);
          black_box(Fiber::new(|_| (), r))
        });
      }
    }
  );
  group.bench_function(
    "safe",
    |b| {
      let p = PageSize::get().unwrap();
      let s = SafeStack::new(8192, p).unwrap();
      b.iter(|| {
        let r = Reusable(&s);
        black_box(Fiber::new(|_| (), r))
      });
    }
  );
  group.bench_function(
    "paranoid",
    |b| {
      let p = PageSize::get().unwrap();
      let s = ParanoidStack::new(8192, p).unwrap();
      b.iter(|| {
        let r = Reusable(&s);
        black_box(Fiber::new(|_| (), r))
      });
    }
  );
}

fn poll_pending(c: &mut Criterion) {
  // These should all take basically the same time, but they will bounce about a bit.
  panic::set_hook(Box::new(|_| ())); // eliminate noise
  let mut group = c.benchmark_group("poll_pending");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "allocator",
    |b| {
      unsafe {
        let s = AllocatorStack::new(8192);
        let s = Fiber::new(closure, s);
        dummy!(s);
        b.iter(|| black_box(s.poll()));
      }
    }
  );
  group.bench_function(
    "safe",
    |b| {
      let p = PageSize::get().unwrap();
      let s = SafeStack::new(8192, p).unwrap();
      let s = Fiber::new(closure, s);
      dummy!(s);
      b.iter(|| black_box(s.poll()));
    }
  );
  group.bench_function(
    "paranoid",
    |b| {
      let p = PageSize::get().unwrap();
      let s = ParanoidStack::new(8192, p).unwrap();
      let s = Fiber::new(closure, s);
      dummy!(s);
      b.iter(|| black_box(s.poll()));
    }
  );
}

criterion_group!(
  benches,
  stack_creation,
  linking,
  poll_pending
);
criterion_main!(benches);
