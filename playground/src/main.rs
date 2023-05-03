use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
    sync::Arc,
};

use cognite::raw::{RetrieveCursorsQuery, RetrieveRowsQuery};
use futures::future::join_all;
use json_transform::Program;
use serde_json::{json, Value};
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() {
    let data: Value = if Path::new("data.json").exists() {
        let file = File::open("data.json").unwrap();
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).unwrap()
    } else {
        let data = download_table().await.unwrap();
        let res = Value::Array(data);
        let file = File::create("data.json").unwrap();
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &res).unwrap();
        res
    };

    let program = Program::compile(
        serde_json::from_value(json!([{
            "id": "build",
            "inputs": ["input"],
            "transform": r#"[
                if(input.temperature, {
                    "externalId": concat(input.stationName, '.', 'temperature'),
                    "time": to_unix_timestamp(input.datetime, '%Y-%m-%d %H:%M:%S%.f'),
                    "value": float(input.temperature)
                }),
                if(input.cloudCoverage, {
                    "externalId": concat(input.stationName, '.', 'cloudCover'),
                    "time": to_unix_timestamp(input.datetime, '%Y-%m-%d %H:%M:%S%.f'),
                    "value": input.cloudCoverage
                }),
                if(input.wintSpeed, {
                    "externalId": concat(input.stationName, '.', 'windspeed'),
                    "time": to_unix_timestamp(input.datetime, '%Y-%m-%d %H:%M:%S%.f'),
                    "value": float(input.wintSpeed)
                })
            ].filter((i) => i)"#,
            "expandOutput": true
        }]))
        .unwrap(),
    )
    .unwrap();

    /* let program = Program::compile(
        serde_json::from_value(json!([{
            "id": "temperature",
            "inputs": ["input"],
            "transform": r#"{
                "externalId": concat($input.stationName, '.', 'temperature'),
                "time": to_unix_timestamp($input.datetime, '%Y-%m-%d %H:%M:%S%.f'),
                "value": if($input.temperature, float($input.temperature)),
            }"#
        }, {
            "id": "cloudcover",
            "inputs": ["input"],
            "transform": r#"{
                "externalId": concat($input.stationName, '.', 'cloudCover'),
                "time": to_unix_timestamp($input.datetime, '%Y-%m-%d %H:%M:%S%.f'),
                "value": $input.cloudCoverage,
            }"#
        }, {
            "id": "windspeed",
            "inputs": ["input"],
            "transform": r#"{
                "externalId": concat($input.stationName, '.', 'windspeed'),
                "time": to_unix_timestamp($input.datetime, '%Y-%m-%d %H:%M:%S%.f'),
                "value": if($input.wintSpeed, float($input.wintSpeed)),
            }"#
        }, {
            "id": "combine",
            "inputs": ["temperature", "cloudcover", "windspeed"],
            "type": "filter",
            "transform": "$merge.value",
            "mode": "merge"
        }]))
        .unwrap(),
    )
    .unwrap(); */
    let start = std::time::Instant::now();
    println!("Begin executing program");
    let mut fin = vec![];
    for it in data.as_array().unwrap() {
        let output = program.execute(&it).unwrap();
        for o in output {
            fin.push(o);
        }
    }
    println!(
        "End executing program: {}ms",
        (std::time::Instant::now() - start).as_millis()
    );
    let file = File::create("output.json").unwrap();
    let writer = BufWriter::new(file);
    serde_json::to_writer(writer, &Value::Array(fin)).unwrap();
}

async fn download_table() -> cognite::Result<Vec<Value>> {
    let client = Arc::new(
        cognite::CogniteClient::new_oidc(
            "jsontftest",
            Some(cognite::ClientConfig {
                max_retries: 5,
                ..Default::default()
            }),
        )
        .unwrap(),
    );
    let cursors = client
        .raw
        .retrieve_cursors_for_parallel_reads(
            "justice_data",
            "measurements",
            Some(RetrieveCursorsQuery {
                min_last_updated_time: None,
                max_last_updated_time: None,
                number_of_cursors: Some(10),
            }),
        )
        .await?;

    let mut futures: Vec<JoinHandle<Result<Vec<Value>, cognite::Error>>> = vec![];
    for cursor in cursors {
        let client = client.clone();
        futures.push(tokio::spawn(async move {
            let mut cs = Some(cursor);
            let mut result = vec![];
            let mut num = 0;
            while cs.is_some() {
                let res = client
                    .raw
                    .retrieve_rows(
                        "justice_data",
                        "measurements",
                        Some(RetrieveRowsQuery {
                            cursor: cs,
                            limit: Some(1000),
                            ..Default::default()
                        }),
                    )
                    .await?;
                num = num + res.items.len();
                println!(
                    "Retrieved {} raw rows results. {} total",
                    res.items.len(),
                    num
                );
                for row in res.items {
                    result.push(row.columns);
                }
                cs = res.next_cursor;
            }
            Ok(result)
        }));
    }
    let mut fin = vec![];
    let results = join_all(futures).await;
    for rs in results {
        let data = rs.unwrap()?;
        for row in data {
            fin.push(row);
        }
    }
    Ok(fin)
}
