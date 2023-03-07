use criterion::{black_box, criterion_group, criterion_main, Criterion};
use manual_analyzer::lexer::lex;

fn bench_lexer(c: &mut Criterion) {
    let mut group =
        c.benchmark_group("Lexing ARM assembly");

    let input = include_str!("radix_sort.s");

    group.throughput(criterion::Throughput::Bytes(input.len() as u64));

    group.bench_function("Lexer", |b| {
        b.iter(|| lex(black_box(input)))
    });
}

criterion_group!(benches, bench_lexer);
criterion_main!(benches);
