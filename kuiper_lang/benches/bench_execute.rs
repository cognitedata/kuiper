use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kuiper_lang::compile_expression;
use serde_json::json;

mod perf;

fn bench_trivial_map(c: &mut Criterion) {
    let expr = compile_expression(
        r#"{
        "externalId": input.id,
        "value": input.val
    }"#,
        &["input"],
    )
    .unwrap();
    let input = json!({
        "id": "my-id",
        "val": 1234
    });

    c.bench_function("trivial map 1", move |f| {
        f.iter(|| expr.run(black_box([&input])).unwrap())
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(perf::FlamegraphProfiler::new(100));
    targets = bench_trivial_map
}
criterion_main!(benches);
