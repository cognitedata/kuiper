use criterion::{black_box, criterion_group, criterion_main, Criterion};
use json_transform::{Program, TransformInput};
use serde_json::json;

fn bench_flatten_map(c: &mut Criterion) {
    let raw: Vec<TransformInput> = serde_json::from_value(json!([
        {
            "id": "step1",
            "inputs": ["input"],
            "transform": "$input.values",
            "type": "flatten"
        },
        {
            "id": "step2",
            "inputs": ["input", "step1"],
            "transform": {
                "externalId": "$input.id",
                "value": "$step1.value * pow(10, $step1.valueExponent)",
                "timestamp": "$step1.time"
            },
            "type": "map"
        }
    ]))
    .unwrap();
    let program = Program::compile(raw).unwrap();
    let input = json!({
        "id": "my-id",
        "values": [{
            "value": 123.123,
            "valueExponent": 5,
            "time": 123142812824u64
        }, {
            "value": 321.321,
            "valueExponent": 5,
            "time": 123901591231u64
        }]
    });

    c.bench_function("flatten map 2", move |f| {
        f.iter(|| program.execute(black_box(&input)).unwrap())
    });
}

fn bench_trivial_map(c: &mut Criterion) {
    let raw: Vec<TransformInput> = serde_json::from_value(json!([
        {
            "id": "step",
            "inputs": ["input"],
            "transform": {
                "externalId": "$input.id",
                "value": "$input.val"
            },
            "type": "map"
        }
    ]))
    .unwrap();
    let program = Program::compile(raw).unwrap();
    let input = json!({
        "id": "my-id",
        "val": 1234
    });

    c.bench_function("trivial map 1", move |f| {
        f.iter(|| program.execute(black_box(&input)).unwrap())
    });
}

fn bench_exponential_flatten(c: &mut Criterion) {
    let raw: Vec<TransformInput> = serde_json::from_value(json!([
        {
            "id": "step1",
            "inputs": ["input"],
            "transform": "$input.values",
            "type": "flatten"
        }, // 2
        {
            "id": "gen",
            "inputs": [],
            "transform": "[0, 1, 2, 3, 4]",
            "type": "flatten"
        }, // 5
        {
            "id": "explode1",
            "inputs": ["gen", "step1"],
            "transform": {
                "v1": "$gen",
                "v2": "$step1.value"
            },
            "type": "map"
        }, // 5 * 2
        {
            "id": "explode2",
            "inputs": ["gen", "explode1"],
            "transform": {
                "v1": "$gen",
                "v21": "$explode1.v1",
                "v22": "$explode1.v2"
            },
            "type": "map"
        }, // 5 * (5 * 2) = 50
        {
            "id": "explode3",
            "inputs": ["explode1", "explode2"],
            "transform": {
                "v11": "$explode1.v1",
                "v12": "$explode2.v1",
                "v21": "$explode2.v21",
                "v22": "$explode2.v22",
                "v23": "$explode1.v2"
            },
            "type": "map"
        } // (5 * 2) * (5 * (5 * 2)) = 500
    ]))
    .unwrap();
    let program = Program::compile(raw).unwrap();
    let input = json!({
        "id": "my-id",
        "values": [{
            "value": 123.123,
            "time": 123142812824u64
        }, {
            "value": 321.321,
            "time": 123901591231u64
        }]
    });

    // Outputs
    c.bench_function("flatten map exp", move |f| {
        f.iter(|| program.execute(black_box(&input)).unwrap())
    });
}

criterion_group!(
    benches,
    bench_flatten_map,
    bench_trivial_map,
    bench_exponential_flatten
);
criterion_main!(benches);
