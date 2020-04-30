use crate::db_context::{create_context, DbContext};
use crate::delta_processor::{add_processor_methods_to_table, apply_delta};
use crate::document::reset_document;
use crate::events::MessageTiming;
use crate::hashtuple::LookupTable;
use crate::parsing::parse;
use crate::properties::insert_properties;
use crate::resources::insert_resources;
use diesel::prelude::*;
use futures::StreamExt;
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::error::KafkaResult;
use rdkafka::{ClientConfig, Message};
use std::collections::HashMap;
use std::io::{stdout, Write};
use std::time::SystemTime;
use tokio::sync::mpsc::Sender;
use tokio::task;

type Timing = (u128, u128, u128);

pub(crate) async fn import(
    updates: &mut Sender<MessageTiming>,
) -> Result<(), Box<dyn std::error::Error>> {
    let consumer = create_kafka_consumer().expect("Failed to create kafka consumer");
    println!("Initialized kafka config");

    let topic = dotenv!("KAFKA_TOPIC");
    consumer.subscribe(&[topic])?;

    let db_conn = PgConnection::establish(dotenv!("DATABASE_URL")).expect("Error connecting to pg");

    //    get_doc_count(db_conn);

    let mut stream = consumer.start();
    let mut i = 0i64;
    let mut parse_time = 0u128;
    let mut fetch_time = 0u128;
    let mut insert_time = 0u128;

    let mut ctx = create_context(&db_conn);
    println!("Start listening for messages");

    while let Some(message) = stream.next().await {
        match message {
            Err(e) => {
                warn!("Kafka error: {}", e);
            }
            Ok(m) => {
                let transaction = db_conn.transaction::<(), diesel::result::Error, _>(|| {
                    let payload = &m.payload().expect("message has no payload");
                    let timing =
                        process_message(&mut ctx, payload).expect("process_message failed");
                    updates.send(timing);

                    // Now that the message is completely processed, add it's position to the offset
                    // store. The actual offset will be committed every 5 seconds.
                    if let Err(e) = consumer.store_offset(&m) {
                        warn!("Error while storing offset: {}", e);
                    }

                    Ok(())
                });

                match transaction {
                    Ok(_) => print!("."),
                    Err(_) => print!("e"),
                };
                i += 1;
                if i > 5 {
                    println!(
                        "\nTiming: parse: {}, fetch/del: {}, insert: {}",
                        parse_time, fetch_time, insert_time
                    );
                    parse_time = 0;
                    fetch_time = 0;
                    insert_time = 0;

                    stdout().flush().expect("Flush to stdout failed");
                    i = 0;
                }
            }
        }

        task::yield_now().await;
    }

    Ok(())
}

pub(crate) fn process_message(ctx: &mut DbContext, payload: &[u8]) -> Result<MessageTiming, ()> {
    let parse_start = SystemTime::now();

    let mut lookup_table: LookupTable = LookupTable::new();
    add_processor_methods_to_table(&mut lookup_table);
    let docs = parse(&mut lookup_table, payload);
    let parse_time = SystemTime::now()
        .duration_since(parse_start)
        .unwrap()
        .as_millis();
    let mut fetch_time = 0;
    let mut insert_time = 0;

    for (k, delta) in docs {
        let fetch_start = SystemTime::now();
        let id = k.parse::<i64>().unwrap();

        let existing = reset_document(&ctx, &mut lookup_table, id);
        fetch_time += SystemTime::now()
            .duration_since(fetch_start)
            .unwrap()
            .as_millis();

        let next = apply_delta(&lookup_table, &existing, &delta);

        //// Replace
        let insert_start = SystemTime::now();
        let resources = insert_resources(ctx.db_conn, &lookup_table, &next, id);
        let mut resource_id_map = HashMap::<String, i64>::new();
        for resource in resources {
            resource_id_map.insert(resource.iri.clone(), resource.id);
        }

        insert_properties(ctx, &lookup_table, &next, resource_id_map);
        insert_time += SystemTime::now()
            .duration_since(insert_start)
            .unwrap()
            .as_millis();
    }

    Ok(MessageTiming {
        parse_time,
        fetch_time,
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
    config.set("queue.buffering.max.ms", "0");
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
