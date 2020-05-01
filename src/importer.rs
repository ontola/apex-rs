use crate::db_context::{create_context, DbContext};
use crate::delta_processor::{add_processor_methods_to_table, apply_delta};
use crate::document::reset_document;
use crate::events::{DeltaProcessingTiming, MessageTiming};
use crate::hashtuple::LookupTable;
use crate::parsing::parse;
use crate::properties::insert_properties;
use crate::resources::insert_resources;
use diesel::prelude::*;
use futures::StreamExt;
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::error::KafkaResult;
use rdkafka::message::BorrowedMessage;
use rdkafka::{ClientConfig, Message};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio::task;

pub(crate) async fn import(updates: &mut Sender<MessageTiming>) -> Result<(), ()> {
    let consumer = create_kafka_consumer().expect("Failed to create kafka consumer");
    println!("Initialized kafka config");

    let topic = dotenv!("KAFKA_TOPIC");
    consumer
        .subscribe(&[topic])
        .expect("Subscribing to topic failed");

    let db_conn = PgConnection::establish(dotenv!("DATABASE_URL")).expect("Error connecting to pg");

    //    get_doc_count(db_conn);

    let mut stream = consumer.start();

    let mut ctx = create_context(&db_conn);
    println!("Start listening for messages");
    let mut last_listen_time = Instant::now();

    while let Some(message) = stream.next().await {
        let msg_poll_time = Instant::now().duration_since(last_listen_time);
        match message {
            Err(e) => {
                warn!("Kafka error: {}", e);
            }
            Ok(m) => {
                let timing = process_message(&mut ctx, &m).await;
                if let Err(e) = consumer.store_offset(&m) {
                    warn!("Error while storing offset: {}", e);
                }
                updates
                    .send(MessageTiming {
                        poll_time: msg_poll_time,
                        ..timing
                    })
                    .await;
            }
        }
        task::yield_now().await;

        last_listen_time = Instant::now();
    }

    Ok(())
}

async fn process_message<'a>(ctx: &mut DbContext<'a>, m: &BorrowedMessage<'a>) -> MessageTiming {
    let mut timing: MessageTiming = MessageTiming::new();

    ctx.db_conn
        .transaction::<(), diesel::result::Error, _>(|| {
            let payload = &m.payload().expect("message has no payload");
            timing = process_delta(ctx, payload).expect("process_message failed");
            Ok(())
        })
        .expect("Error while processing message");

    timing
}

pub(crate) fn process_delta<'a>(
    ctx: &mut DbContext<'a>,
    payload: &[u8],
) -> Result<MessageTiming, ()> {
    let parse_start = Instant::now();

    let mut lookup_table: LookupTable = LookupTable::new();
    add_processor_methods_to_table(&mut lookup_table);
    let docs = parse(&mut lookup_table, payload);
    let parse_time = Instant::now().duration_since(parse_start);
    let mut fetch_time = Duration::new(0, 0);
    let mut delta_time = DeltaProcessingTiming::new();
    let mut insert_time = Duration::new(0, 0);

    for (k, delta) in docs {
        let fetch_start = Instant::now();
        let id = k.parse::<i64>().unwrap();

        let existing = reset_document(&ctx, &mut lookup_table, id);
        fetch_time += Instant::now().duration_since(fetch_start);

        let (next, delta_timing) = apply_delta(&lookup_table, &existing, &delta);
        delta_time += delta_timing;

        let insert_start = Instant::now();
        let resources = insert_resources(ctx.db_conn, &lookup_table, &next, id);
        let mut resource_id_map = HashMap::<String, i64>::new();
        for resource in resources {
            resource_id_map.insert(resource.iri.clone(), resource.id);
        }

        insert_properties(ctx, &lookup_table, &next, resource_id_map);
        insert_time += Instant::now().duration_since(insert_start);
    }

    Ok(MessageTiming {
        poll_time: Duration::new(0, 0),
        parse_time,
        fetch_time,
        delta_time,
        insert_time,
    })
}

fn create_kafka_consumer() -> KafkaResult<StreamConsumer> {
    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", dotenv!("KAFKA_ADDRESS"));

    config.set("sasl.mechanisms", "PLAIN");
    config.set("security.protocol", "SASL_SSL");
    config.set("sasl.username", dotenv!("KAFKA_USERNAME"));
    config.set("sasl.password", dotenv!("KAFKA_PASSWORD"));

    config.set("group.id", dotenv!("KAFKA_GROUP_ID"));
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
