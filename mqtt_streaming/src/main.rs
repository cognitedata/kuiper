use std::{collections::HashMap, sync::Arc, time::Duration};

use chrono::Utc;
use cognite::{
    time_series::{
        AddTimeSerie, DataPointInsertionItem, DataPointInsertionRequest, DatapointType,
        IdOrExternalId, NumericDatapoint, NumericDatapoints, StringDatapoint, StringDatapoints,
    },
    Identity,
};
use json_transform::Program;
use paho_mqtt::{ConnectOptionsBuilder, MessageBuilder};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let queue = Arc::new(RwLock::new(Uploader::new()));
    tokio::spawn(generate());
    let m_queue = queue.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            m_queue.write().await.push().await.unwrap();
        }
    });

    let mut client = paho_mqtt::AsyncClient::new("tcp://localhost:1881").unwrap();
    let stream = client.get_stream(100);
    let opts = ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(10))
        .clean_session(true)
        .finalize();
    client.connect(opts).await.unwrap();
    client.subscribe("mytopic", 2).await.unwrap();

    let transform = Program::compile(
        serde_json::from_value(json!([{
            "id": "split",
            "inputs": ["input"],
            "type": "flatten",
            "transform": "pairs($input)"
        }, {
            "id": "drop_time",
            "inputs": ["split"],
            "transform": "$split.key != 'time'",
            "type": "filter"
        }, {
            "id": "to_outputs",
            "inputs": ["drop_time", "input"],
            "transform": {
                "value": "$drop_time.value",
                "externalId": "concat('mqtt_stream.', $drop_time.key)",
                "timestamp": "to_unix_timestamp($input.time, '%Y-%m-%dT%H:%M:%S%.fZ')"
            },
            "type": "map"
        }]))
        .unwrap(),
    )
    .unwrap();

    loop {
        let msg = stream.recv().await.unwrap();
        if let Some(msg) = msg {
            let raw: Value = serde_json::from_slice(msg.payload()).unwrap();
            let output = transform.execute(&raw).unwrap();
            let mut upl = queue.write().await;
            for msg in output {
                upl.insert(msg).unwrap();
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum NumberOrString {
    Float(f64),
    String(String),
}

impl NumberOrString {
    pub fn into_number(self) -> f64 {
        match self {
            Self::Float(x) => x,
            Self::String(_) => panic!("Got string, expected number"),
        }
    }

    pub fn into_string(self) -> String {
        match self {
            Self::Float(x) => x.to_string(),
            Self::String(s) => s,
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DatapointWithId {
    pub external_id: String,
    #[serde(flatten)]
    pub dp: Datapoint,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Datapoint {
    pub value: NumberOrString,
    pub timestamp: i64,
}

struct Uploader {
    queue: HashMap<String, Vec<Datapoint>>,
    is_string: HashMap<String, bool>,
    client: cognite::CogniteClient,
}

impl Uploader {
    pub fn new() -> Self {
        Self {
            queue: HashMap::new(),
            is_string: HashMap::new(),
            client: cognite::CogniteClient::new_oidc("mqtt_test_rs", None).unwrap(),
        }
    }
    pub fn insert(&mut self, val: Value) -> Result<(), serde_json::Error> {
        let dp: DatapointWithId = serde_json::from_value(val)?;
        match self.queue.entry(dp.external_id) {
            std::collections::hash_map::Entry::Occupied(mut x) => {
                x.get_mut().push(dp.dp);
            }
            std::collections::hash_map::Entry::Vacant(x) => {
                x.insert(vec![dp.dp]);
            }
        }

        Ok(())
    }
    pub async fn push(&mut self) -> Result<(), cognite::Error> {
        let mut items = vec![];
        let mut total = 0;
        let mut total_ts = 0;
        for (id, dps) in self.queue.drain() {
            total_ts += 1;
            total += dps.len();
            let dps = if dps.first().unwrap().value.is_string() {
                if !self.is_string.contains_key(&id) {
                    self.is_string.insert(id.clone(), true);
                }
                let ins = dps
                    .into_iter()
                    .map(|d| StringDatapoint {
                        value: d.value.into_string(),
                        timestamp: d.timestamp,
                    })
                    .collect();
                DatapointType::StringDatapoints(StringDatapoints { datapoints: ins })
            } else {
                if !self.is_string.contains_key(&id) {
                    self.is_string.insert(id.clone(), false);
                }
                let ins = dps
                    .into_iter()
                    .map(|d| NumericDatapoint {
                        value: d.value.into_number(),
                        timestamp: d.timestamp,
                    })
                    .collect();
                DatapointType::NumericDatapoints(NumericDatapoints { datapoints: ins })
            };

            let it = DataPointInsertionItem {
                id_or_external_id: Some(IdOrExternalId::ExternalId(id)),
                datapoint_type: Some(dps),
            };
            items.push(it)
        }

        println!(
            "Uploading {} datapoints for {} timeseries to CDF",
            total, total_ts
        );

        self.client
            .time_series
            .insert_datapoints_proto_create_missing(
                &DataPointInsertionRequest { items },
                &|idts: &[Identity]| {
                    idts.iter()
                        .map(|i| {
                            let extid = match i {
                                Identity::ExternalId { external_id } => external_id,
                                _ => unreachable!(),
                            };
                            AddTimeSerie {
                                external_id: Some(extid.clone()),
                                is_string: self.is_string.get(extid).cloned().unwrap_or_default(),
                                ..Default::default()
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_iter()
                },
            )
            .await?;
        Ok(())
    }
}

struct SimState {
    pub value: f64,
    pub id: String,
}

impl SimState {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            value: 0f64,
        }
    }
}

async fn generate() {
    let mut states = [
        SimState::new("sin"),
        SimState::new("cos"),
        SimState::new("inc"),
    ];
    let client = paho_mqtt::AsyncClient::new("tcp://localhost:1881").unwrap();
    let opts = ConnectOptionsBuilder::new()
        .keep_alive_interval(Duration::from_secs(10))
        .clean_session(true)
        .finalize();
    client.connect(opts).await.unwrap();

    let mut idx = 0;
    loop {
        let tick = (idx as f64) / 100f64;
        states[0].value = tick.sin();
        states[1].value = tick.cos();
        states[2].value = tick;
        idx += 1;

        let msg = json!({
            "sin": states[0].value,
            "cos": states[1].value,
            "inc": states[2].value,
            "nsin": -states[0].value,
            "ncos": -states[1].value,
            "ninc": -states[2].value,
            "time": Utc::now()
        });

        // println!("Publish message {}", &msg);

        client
            .publish(
                MessageBuilder::new()
                    .topic("mytopic")
                    .payload(serde_json::to_vec(&msg).unwrap())
                    .finalize(),
            )
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
