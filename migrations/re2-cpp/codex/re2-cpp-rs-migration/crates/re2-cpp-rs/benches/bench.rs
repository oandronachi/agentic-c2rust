use criterion::{black_box, criterion_group, criterion_main, Criterion};
use re2_cpp_rs::Regex;

fn bench_compile_literal(c: &mut Criterion) {
    c.bench_function("compile_literal", |b| {
        b.iter(|| Regex::new(black_box("needle")).unwrap())
    });
}

fn bench_partial_match(c: &mut Criterion) {
    let re = Regex::new("needle").unwrap();
    c.bench_function("partial_match", |b| {
        b.iter(|| re.partial_match(black_box("hay needle stack")))
    });
}

criterion_group!(benches, bench_compile_literal, bench_partial_match);
criterion_main!(benches);
