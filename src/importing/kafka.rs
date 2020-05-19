use crate::db::db_context::DbContext;
use crate::errors::ErrorKind;
use crate::importing::events::MessageTiming;
use crate::importing::importer::process_message;
use crate::importing::parsing::parse_nquads;
use futures::StreamExt;
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::error::KafkaResult;
use rdkafka::{ClientConfig, Message};
use std::env;
use std::time::Instant;
use tokio::sync::mpsc::Sender;
use tokio::task;

pub async fn import_kafka(
    updates: &mut Sender<Result<MessageTiming, ErrorKind>>,
) -> Result<(), ()> {
    let consumer = create_kafka_consumer().expect("Failed to create kafka consumer");
    println!("Initialized kafka config");

    consumer
        .subscribe(&[env::var("KAFKA_TOPIC").unwrap().as_str()])
        .expect("Subscribing to topic failed");

    let mut stream = consumer.start();

    let pool = DbContext::default_pool();
    let mut ctx = DbContext::new(&pool);
    println!("Start listening for messages");
    let mut last_listen_time = Instant::now();

    while let Some(message) = stream.next().await {
        let msg_poll_time = Instant::now().duration_since(last_listen_time);

        let t = match message {
            Err(e) => {
                warn!("Kafka error: {}", e);
                Err(ErrorKind::Unexpected)
            }
            Ok(msg) => {
                if let Some(payload) = msg.payload() {
                    match parse_nquads(
                        &mut ctx.lookup_table,
                        &String::from_utf8(Vec::from(payload)).unwrap(),
                    ) {
                        Ok(model) => match process_message(&mut ctx, model).await {
                            Ok(timing) => {
                                if let Err(e) = consumer.store_offset(&msg) {
                                    warn!("Error while storing offset: {}", e);
                                    Err(ErrorKind::Commit)
                                } else {
                                    Ok(MessageTiming {
                                        poll_time: msg_poll_time,
                                        ..timing
                                    })
                                }
                            }
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e),
                    }
                } else {
                    error!(target: "apex", "message has no payload");
                    Err(ErrorKind::EmptyDelta)
                }
            }
        };
        if let Err(e) = updates.send(t).await {
            error!(target: "apex", "Error while sending result to reporter: {}", e);
        }
        task::yield_now().await;

        last_listen_time = Instant::now();
    }

    Ok(())
}

fn create_kafka_consumer() -> KafkaResult<StreamConsumer> {
    let mut config = ClientConfig::new();
    config.set(
        "bootstrap.servers",
        env::var("KAFKA_ADDRESS").unwrap().as_str(),
    );

    config.set("sasl.mechanisms", "PLAIN");
    config.set("security.protocol", "SASL_SSL");
    config.set(
        "sasl.username",
        env::var("KAFKA_USERNAME").unwrap().as_str(),
    );
    config.set(
        "sasl.password",
        env::var("KAFKA_PASSWORD").unwrap().as_str(),
    );

    config.set("group.id", env::var("KAFKA_GROUP_ID").unwrap().as_str());
    // config.set("queue.buffering.max.ms", "0");
    //    config.set("allow.auto.create.topics", "false");
    config.set("fetch.max.bytes", "5428800");
    //    config.set("fetch.max.wait.ms", "1000");
    //    config.set("max.poll.records", "500");
    config.set("session.timeout.ms", "60000");
    config.set("enable.auto.commit", "true");
    config.set("auto.commit.interval.ms", "1000");
    config.set("request.timeout.ms", "20000");
    config.set("retry.backoff.ms", "500");

    config.set_log_level(RDKafkaLogLevel::Debug);

    config.create::<StreamConsumer>()
}
