use anyhow::anyhow;
use anyhow::Result;
use fe2o3_amqp::types::messaging::Body;
use fe2o3_amqp::types::primitives::Value;
use fe2o3_amqp::Receiver;
use fe2o3_amqp::{
    connection::ConnectionHandle, sasl_profile::SaslProfile, types::primitives::Array, Connection,
    Session,
};
use fe2o3_amqp_management::{operations::ReadRequest, MgmtClient};
use json_transform::Program;
use rumqttc::v5::MqttOptions;
use serde_json::json;
use std::env;

#[tokio::main]
async fn main() {
    let hostname = env::var("AMQP_HOST").unwrap();
    let sa_key_name = env::var("AMQP_KEY_NAME").unwrap();
    let sa_key_value = env::var("AMQP_KEY_VALUE").unwrap();
    let event_hub_name = env::var("EVENT_HUB_NAME").unwrap();

    let url = format!("amqps://{hostname}");

    let mut connection = Connection::builder()
        .container_id("rust-connection-1")
        .alt_tls_establishment(true)
        .sasl_profile(SaslProfile::Plain {
            username: sa_key_name.to_string(),
            password: sa_key_value.to_string(),
        })
        .open(&url[..])
        .await
        .unwrap();

    let partitions = get_event_hub_partitions(&mut connection, &event_hub_name)
        .await
        .unwrap();

    println!("Partitions:");
    for p in &partitions {
        println!("    {p}");
    }

    let partition = &partitions[0];
    // let partition = 0;
    let partition_address =
        format!("{event_hub_name}/ConsumerGroups/$default/Partitions/{partition}");

    let mut session = Session::begin(&mut connection).await.unwrap();

    let mut receiver = Receiver::attach(
        &mut session,
        format!("receiver-{}", partition),
        partition_address,
    )
    .await
    .unwrap();

    // stream_cdf(&mut receiver).await;
    stream_mqtt(&mut receiver).await;

    receiver.close().await.unwrap();
    session.end().await.unwrap();
    connection.close().await.unwrap();
}

async fn stream_mqtt(receiver: &mut Receiver) {
    let options = MqttOptions::new("amqp_pub", "localhost", 1883);

    let (client, mut event_loop) = rumqttc::v5::AsyncClient::new(options, 1000);

    tokio::spawn(async move {
        loop {
            event_loop.poll().await.unwrap();
        }
    });

    let mut idx = 0;
    loop {
        println!("Begin waiting for amqp delivery");
        let delivery = receiver.recv::<Body<Value>>().await.unwrap();
        println!("Received amqp batch");
        for dt in delivery.body().try_as_data().unwrap() {
            client
                .publish(
                    "example/message",
                    rumqttc::v5::mqttbytes::QoS::AtLeastOnce,
                    false,
                    dt.clone().into_vec(),
                )
                .await
                .unwrap();
            println!("Received message {}", idx);
            idx += 1;
        }
        println!("");
        receiver.accept(&delivery).await.unwrap();
    }
}

async fn stream_cdf(receiver: &mut Receiver) {
    let transform = Program::compile(
        serde_json::from_value(json!([{
            "id": "get_items",
            "inputs": ["input"],
            "type": "flatten",
            "transform": "$input.gatewayData"
        }, {
            "id": "group",
            "inputs": ["get_items"],
            "transform": "flatten($get_items.vqts, $get_items)",
            "type": "flatten"
        }, {
            "id": "to_outputs",
            "inputs": ["group"],
            "transform": {
                "value": "float($group.v)",
                "externalId": "concat('eventhub.', $group.tag_id)",
                "timestamp": "to_unix_timestamp($group.t, '%Y-%m-%dT%H:%M:%S%.fZ')"
            },
            "type": "map"
        }]))
        .unwrap(),
    )
    .unwrap();

    for _ in 0..3 {
        let delivery = receiver.recv::<Body<Value>>().await.unwrap();
        for dt in delivery.body().try_as_data().unwrap() {
            let value: serde_json::Value = serde_json::from_slice(&dt).unwrap();
            let res = transform.execute(&value).unwrap();
            for val in &res {
                println!("{}", val);
            }
        }
        println!("");
        receiver.accept(&delivery).await.unwrap();
    }
}

pub async fn get_event_hub_partitions(
    connection: &mut ConnectionHandle<()>,
    event_hub_name: &str,
) -> Result<Vec<String>> {
    let mut session = Session::begin(connection).await.unwrap();
    let mut mgmt_client = MgmtClient::attach(&mut session, "mgmt_client_node")
        .await
        .unwrap();

    println!("Evhub: {}", event_hub_name);
    let request = ReadRequest::name(event_hub_name, "com.microsoft:eventhub", None);
    let mut response = mgmt_client.call(request).await.unwrap();

    mgmt_client.close().await?;
    session.end().await?;

    let partition_value = response
        .entity_attributes
        .remove("partition_ids")
        .ok_or(anyhow!("partition_ids not found"))?;

    let partitions: Array<String> = partition_value
        .try_into()
        .map_err(|val| anyhow!("Invalid partitions value {:?}", val))?;
    Ok(partitions.into_inner())
}
