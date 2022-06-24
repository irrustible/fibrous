use criterion::*;
use fibrous::{*, stack::*};
use std::future::pending;
use std::panic;
use wookie::dummy;

struct Reusable<'a>(&'a AllocatorStackConst<8192>);

// This is going to help us get more accurate measurements.
// Absolutely do *not* do this in a real program, it's a very dumb idea.
unsafe impl<'a> Stack for Reusable<'a> {
  fn end(&self) -> *mut usize { self.0.end() }
}

fn closure(awaiter: &Awaiter) {
  loop { awaiter.wait(pending::<()>()); }
}

fn linking(c: &mut Criterion) {
  let mut group = c.benchmark_group("linking_closure");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "green-threads",
    |b| {
      panic::set_hook(Box::new(|_| ())); // eliminate noise
      unsafe {
        let s = AllocatorStackConst::<8192>::new();
        b.iter(|| {
          let r = Reusable(&s);
          black_box(Fiber::new(|_| (), r))
        });
      }
    }
  );
}

fn poll_pending(c: &mut Criterion) {
  let mut group = c.benchmark_group("poll_pending");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "green-threads",
    |b| {
      panic::set_hook(Box::new(|_| ())); // eliminate noise
      unsafe {
        let s = AllocatorStackConst::<8192>::new();
        let s = Fiber::new(closure, s);
        dummy!(s);
        b.iter(|| black_box(s.poll()));
      }
    }
  );
}

criterion_group!(
  benches,
  linking,
  poll_pending
);
criterion_main!(benches);
