use crate::app_config::AppConfig;
use crate::db::db_context::DbContext;
use crate::db::document::delete_document_data;
use crate::errors::ErrorKind;
use crate::importing::events::MessageTiming;
use crate::importing::importer::process_invalidate;
use crate::importing::parsing::{parse_hndjson, DocumentSet};
use diesel::Connection;
use log::Level;
use std::env;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio::task;

pub async fn invalidator_redis(
    updates: &mut Sender<Result<MessageTiming, ErrorKind>>,
) -> Result<(), String> {
    let mut consumer = create_redis_consumer().expect("Failed to create redis consumer");
    println!("Initialized redis config");

    let config = AppConfig::default();
    let pool = DbContext::default_pool(config.database_url)?;
    let mut ctx = DbContext::new(&pool);

    let mut pubsub = consumer.as_pubsub();
    let channel = env::var("CACHE_CHANNEL").expect("No redis channel set");
    pubsub
        .subscribe(&channel)
        .unwrap_or_else(|_| panic!("Failed to connect to channel: {}", &channel));
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
                        if log_enabled!(Level::Trace) {
                            trace!(
                                "Recieved message:\n >>>>>{}<<<<<",
                                String::from_utf8(p.clone()).expect("Invalid message")
                            );
                        }
                        let report = match parse_hndjson(&mut ctx.lookup_table, p.as_slice()) {
                            Ok(model) => {
                                let result = if is_invalidate_all_cmd(&mut ctx, &model) {
                                    process_invalidate(&mut ctx).await
                                } else {
                                    process_message(&mut ctx, model).await
                                };

                                match result {
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

pub(crate) async fn process_message(
    ctx: &mut DbContext<'_>,
    docs: DocumentSet,
) -> Result<MessageTiming, ErrorKind> {
    ctx.db_pool
        .get()
        .unwrap()
        .transaction::<MessageTiming, diesel::result::Error, _>(|| {
            for (iri, _) in docs {
                trace!(target: "apex", "Invalidating resource: {}", iri);
                delete_document_data(&ctx.get_conn(), &iri);

                let https_iri = iri.replace("http://", "https://");
                delete_document_data(&ctx.get_conn(), &https_iri);
            }

            Ok(MessageTiming::new())
        })
        .map_err(|_| ErrorKind::Unexpected)
}

pub(crate) fn create_redis_consumer() -> redis::RedisResult<redis::Connection> {
    let client = redis::Client::open(env::var("REDIS_URL").unwrap_or("redis://127.0.0.1/".into()))?;
    client.get_connection()
}

fn is_invalidate_all_cmd(ctx: &mut DbContext, model: &DocumentSet) -> bool {
    if model.len() > 1 {
        return false;
    }

    let var = model.get("http://spinrdf.org/sp#Variable");
    if var.is_none() || var.unwrap().len() != 1 {
        return false;
    }

    let var_uu128 = ctx
        .lookup_table
        .ensure_value("http://spinrdf.org/sp#Variable");
    let inval_uu128 = ctx
        .lookup_table
        .ensure_value("https://ns.ontola.io/core#invalidate");
    let q = var.unwrap().get(0).unwrap();

    q.subject == var_uu128
        && q.predicate == var_uu128
        && q.value == var_uu128
        && q.graph == inval_uu128
}