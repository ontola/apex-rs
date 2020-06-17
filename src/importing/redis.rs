use crate::db::db_context::DbContext;
use crate::errors::ErrorKind;
use crate::importing::events::MessageTiming;
use crate::importing::importer::process_message;
use crate::importing::parsing::parse_hndjson;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio::task;

pub const CHANNEL: &str = "cache";

pub async fn import_redis(
    updates: &mut Sender<Result<MessageTiming, ErrorKind>>,
) -> Result<(), ()> {
    let mut redis_client = create_redis_connection().expect("Failed to create redis consumer");
    println!("Initialized redis config");

    let pool = DbContext::default_pool();
    let mut ctx = DbContext::new(&pool);

    let mut pubsub = redis_client.as_pubsub();
    pubsub
        .subscribe(CHANNEL)
        .unwrap_or_else(|_| panic!("Failed to connect to channel: {}", CHANNEL));
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
                        if let Err(e) = updates.send(Err(ErrorKind::Unexpected)).await {
                            error!(target: "apex", "Error while sending unexpected error to reporter: {}", e);
                        }
                        continue;
                    }
                    Ok(p) => {
                        let report = match parse_hndjson(&mut ctx.lookup_table, p.as_slice()) {
                            Ok(model) => match process_message(&mut ctx, model).await {
                                Ok(timing) => Ok(MessageTiming {
                                    poll_time: msg_poll_time,
                                    ..timing
                                }),
                                Err(e) => Err(e),
                            },
                            Err(e) => {
                                error!(target: "apex", "Unexpected error: {}", e.description());
                                Err(e)
                            }
                        };

                        if let Err(e) = updates.send(report).await {
                            error!(target: "apex", "Error while sending stats to reporter: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    task::yield_now().await;
                    continue;
                }

                if let Err(e) = updates.send(Err(ErrorKind::Unexpected)).await {
                    error!(target: "apex", "Error while sending error to reporter: {}", e);
                }
            }
        }

        task::yield_now().await;
    }
}

pub fn create_redis_connection() -> redis::RedisResult<redis::Connection> {
    let address = match dotenv::var("REDIS_ADDRESS") {
        Ok(line) => line,
        Err(_) => String::from("redis://127.0.0.1/"),
    };
    let client = redis::Client::open(address)?;
    client.get_connection()
}
