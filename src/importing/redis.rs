use crate::db::db_context::DbContext;
use crate::errors::ErrorKind;
use crate::hashtuple::LookupTable;
use crate::importing::events::MessageTiming;
use crate::importing::importer::process_message;
use crate::importing::parsing::parse_hndjson;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio::task;

pub async fn import_redis(
    updates: &mut Sender<Result<MessageTiming, ErrorKind>>,
) -> Result<(), ()> {
    let mut consumer = create_redis_consumer().expect("Failed to create redis consumer");
    println!("Initialized redis config");

    let pool = DbContext::default_pool();
    let mut ctx = DbContext::new(&pool);

    let mut pubsub = consumer.as_pubsub();
    let channel = "cache";
    pubsub
        .subscribe(channel)
        .unwrap_or_else(|_| panic!("Failed to connect to channel: {}", channel));
    pubsub
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();

    loop {
        let last_listen_time = Instant::now();

        match pubsub.get_message() {
            Ok(msg) => {
                let msg_poll_time = Instant::now().duration_since(last_listen_time);
                match msg.get_payload::<Vec<u8>>() {
                    Err(_) => {
                        updates.send(Err(ErrorKind::Unexpected)).await;
                        continue;
                    }
                    Ok(p) => {
                        let mut lookup_table: LookupTable = LookupTable::default();
                        let report = match parse_hndjson(&mut lookup_table, p.as_slice()) {
                            Ok(model) => {
                                match process_message(&mut ctx, &mut lookup_table, model).await {
                                    Ok(timing) => Ok(MessageTiming {
                                        poll_time: msg_poll_time,
                                        ..timing
                                    }),
                                    Err(e) => Err(e),
                                }
                            }
                            Err(e) => {
                                error!(target: "apex", "Unexpected error: {}", e.description());
                                Err(e)
                            }
                        };

                        updates.send(report).await;
                    }
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    task::yield_now().await;
                    continue;
                }

                updates.send(Err(ErrorKind::Unexpected)).await;
            }
        }

        task::yield_now().await;
    }
}

fn create_redis_consumer() -> redis::RedisResult<redis::Connection> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    client.get_connection()
}