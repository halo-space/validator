use criterion::{Criterion, criterion_group, criterion_main};
use halo::validator::prelude::*;
use std::hint::black_box;

fn validation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let validator = Validator::new();
    c.bench_function("value/email", |bench| {
        bench.iter(|| runtime.block_on(validator.value(black_box(&"user@example.com"), "email")))
    });
    c.bench_function("value/required", |bench| {
        bench.iter(|| runtime.block_on(validator.value(black_box(&"value"), "required")))
    });
}

criterion_group!(benches, validation);
criterion_main!(benches);
