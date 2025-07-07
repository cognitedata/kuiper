use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
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

fn bench_cognite_format(c: &mut Criterion) {
    let expr = compile_expression(
        r#"
            input.timeseriesData.flatmap(ts => ts.items.map(k => {
                "externalId": concat(context.topic, "/", ts.externalId),
                "timestamp": k.timestamp,
                "value": k.value,
                "type": "datapoint"
            }))
        "#,
        &["input", "context"],
    )
    .unwrap();

    let input = json!({
        "version": "1.0",
        "timeseriesData": [
            {
                "externalId": "my-timeseries",
                "items": [
                    {
                        "timestamp": 1676550645000i64,
                        "value": 1.6
                    },
                    {
                        "timestamp": 1676550663020i64,
                        "value": 5.6
                    },
                    {
                        "timestamp": 1676550675900i64,
                        "value": 2.4
                    },
                    {
                        "timestamp": 1676550712100i64,
                        "value": 3.1
                    }
                ]
            },
            {
                "externalId": "my-other-timeseries",
                "items": [
                    {
                        "timestamp": 1676550645000i64,
                        "value": "on"
                    },
                    {
                        "timestamp": 1676550663020i64,
                        "value": "off"
                    }
                ]
            }
        ]
    });
    let context = json!({
        "topic": "my/topic"
    });

    c.bench_function("cognite format", move |f| {
        f.iter(|| expr.run(black_box([&input, &context])).unwrap())
    });
}

fn bench_rockwell_format(c: &mut Criterion) {
    let expr = compile_expression(
        r#"
        input.gatewayData.flatmap(ts => ts.vqts.map(k => {
            "externalId": concat(context.topic, "/", ts.tag_id),
            "timestamp": to_unix_timestamp(k.t, "%Y-%m-%dT%T%.3fZ"),
            "value": try_float(k.v, string(k.v)),
            "type": "datapoint"
        }))
        "#,
        &["input", "context"],
    )
    .unwrap();
    let input = json!({
        "gatewayData": [
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Random8",
                "model_id": "kepware1.kepware.Random.Simulator.SimDev1.Random8",
                "vqts": [
                    {
                        "v": "26217",
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/ulint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Random7",
                "model_id": "kepware1.kepware.Random.Simulator.SimDev1.Random7",
                "vqts": [
                    {
                        "v": "5349",
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/ulint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Random6",
                "model_id": "kepware1.kepware.Random.Simulator.SimDev1.Random6",
                "vqts": [
                    {
                        "v": "-979739",
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/lint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Random5",
                "model_id": "kepware1.kepware.Random.Simulator.SimDev1.Random5",
                "vqts": [
                    {
                        "v": -999975268,
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/dint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Random4",
                "model_id": "kepware1.kepware.Random.Simulator.SimDev1.Random4",
                "vqts": [
                    {
                        "v": -370,
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/dint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Random3",
                "model_id": "kepware1.kepware.Random.Simulator.SimDev1.Random3",
                "vqts": [
                    {
                        "v": -608,
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/dint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Random2",
                "model_id": "kepware1.kepware.Random.Simulator.SimDev1.Random2",
                "vqts": [
                    {
                        "v": 253,
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/uint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Random1",
                "model_id": "kepware1.kepware.Random.Simulator.SimDev1.Random1",
                "vqts": [
                    {
                        "v": 39,
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/int"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Ramp8",
                "model_id": "kepware1.kepware.Ramp.Simulator.SimDev1.Ramp8",
                "vqts": [
                    {
                        "v": 152.25,
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/lreal"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Ramp7",
                "model_id": "kepware1.kepware.Ramp.Simulator.SimDev1.Ramp7",
                "vqts": [
                    {
                        "v": "-200746580",
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/lint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Ramp6",
                "model_id": "kepware1.kepware.Ramp.Simulator.SimDev1.Ramp6",
                "vqts": [
                    {
                        "v": "720242500",
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/ulint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Ramp5",
                "model_id": "kepware1.kepware.Ramp.Simulator.SimDev1.Ramp5",
                "vqts": [
                    {
                        "v": "-879000",
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/lint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Ramp4",
                "model_id": "kepware1.kepware.Ramp.Simulator.SimDev1.Ramp4",
                "vqts": [
                    {
                        "v": 60,
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/dint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Ramp3",
                "model_id": "kepware1.kepware.Ramp.Simulator.SimDev1.Ramp3",
                "vqts": [
                    {
                        "v": 32,
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/uint"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Ramp2",
                "model_id": "kepware1.kepware.Ramp.Simulator.SimDev1.Ramp2",
                "vqts": [
                    {
                        "v": 163.75,
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/real"
            },
            {
                "tag_id": "ra-opcda://driver-opcda/Kepware.KEPServerEnterprise/Simulator.SimDev1.Ramp1",
                "model_id": "kepware1.kepware.Ramp.Simulator.SimDev1.Ramp1",
                "vqts": [
                    {
                        "v": 39,
                        "q": 192,
                        "t": "2023-01-26T13:28:13.126Z"
                    }
                ],
                "mimeType": "x-ra/cip/dint"
            }
        ]
    });
    let context = json!({
        "topic": "my/topic"
    });

    c.bench_function("rockwell format", move |f| {
        f.iter(|| expr.run(black_box([&input, &context])).unwrap())
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(perf::FlamegraphProfiler::new(100));
    targets = bench_trivial_map, bench_cognite_format, bench_rockwell_format
}
criterion_main!(benches);
