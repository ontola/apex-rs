use crate::app_config::AppConfig;
use crate::db::db_context::DbContext;
use crate::db::document::delete_document_data;
use crate::errors::ErrorKind;
use crate::importing::events::MessageTiming;
use crate::importing::importer::process_invalidate;
use crate::importing::parsing::{parse_hndjson, DocumentSet};
use crate::importing::redis::create_redis_consumer;
use diesel::Connection;
use log::Level;
use redis::ConnectionLike;
use std::env;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio::task;

pub async fn invalidator_redis(
    updates: &mut Sender<Result<MessageTiming, ErrorKind>>,
) -> Result<(), String> {
    'connection: loop {
        let mut consumer = match create_redis_consumer() {
            Ok(c) => c,
            Err(e) => {
                warn!(target: "apex", "Failed to create redis consumer: {}", e);
                continue;
            }
        };
        println!("Initialized redis config");

        if consumer.is_open() {
            println!("Connected to redis");
        } else {
            println!("Not connected to redis");
        }

        let config = AppConfig::default();
        let pool = match DbContext::default_pool(config.database_url, config.database_pool_size) {
            Ok(p) => p,
            Err(e) => {
                warn!(target: "apex", "Cannot connect to database: {}", e);
                task::yield_now().await;

                continue 'connection;
            }
        };
        let mut ctx = DbContext::new(&pool);

        if let Err(e) = ctx.db_pool.get_timeout(Duration::from_secs(10_000)) {
            warn!("Error connecting to db {}", e);
            task::yield_now().await;

            continue 'connection;
        }

        let mut pubsub = consumer.as_pubsub();
        let channel = env::var("CACHE_CHANNEL").expect("No redis channel set");
        if let Err(e) = pubsub.subscribe(&channel) {
            error!("Failed to connect to channel '{}': {}", &channel, e);
            task::yield_now().await;

            continue 'connection;
        }
        if let Err(e) = pubsub.set_read_timeout(Some(Duration::from_millis(2000))) {
            error!("Failed to read timeout: {}", e);
            task::yield_now().await;

            continue 'connection;
        }

        loop {
            let last_listen_time = Instant::now();

            match pubsub.get_message() {
                Ok(msg) => {
                    let msg_poll_time = Instant::now().duration_since(last_listen_time);
                    match msg.get_payload::<Vec<u8>>() {
                        Err(e) => {
                            if let Err(e) = updates
                                .send(Err(ErrorKind::Unexpected(e.to_string())))
                                .await
                            {
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

                    let mut reconnect = false;

                    if e.is_connection_dropped() {
                        warn!(target: "apex", "Redis connection dropped ({})", e);
                        reconnect = true;
                    } else if e.is_connection_refusal() {
                        warn!(target: "apex", "Redis connection refused ({})", e);
                        reconnect = true;
                    } else if e.is_cluster_error() {
                        warn!(target: "apex", "Redis cluster error ({})", e);
                        reconnect = true;
                    } else if e.is_io_error() {
                        warn!(target: "apex", "Redis IO error, trying to resubscribe ({})", e);
                        reconnect = true;
                    } else {
                        error!(target: "apex", "Unknown redis error: {}", e);
                    }

                    if reconnect == true {
                        warn!(target: "apex", "Reconnecting..");
                        task::yield_now().await;
                        continue 'connection;
                    }

                    task::yield_now().await;

                    if let Err(e) = updates.send(Err(ErrorKind::Unhandled(e.to_string()))).await {
                        error!(target: "apex", "Error while sending error to reporter: {}", e);
                    }
                }
            }

            task::yield_now().await;
        }
    }
}

pub(crate) async fn process_message(
    ctx: &mut DbContext<'_>,
    docs: DocumentSet,
) -> Result<MessageTiming, ErrorKind> {
    ctx.db_pool
        .get()
        .map_err(|e| ErrorKind::Unhandled(e.to_string()))?
        .transaction::<MessageTiming, diesel::result::Error, _>(|| {
            for (iri, _) in docs {
                trace!(target: "apex", "Invalidating resource: {}", iri);
                delete_document_data(&ctx.get_conn(), &iri);

                let https_iri = iri.replace("http://", "https://");
                delete_document_data(&ctx.get_conn(), &https_iri);
            }

            Ok(MessageTiming::new())
        })
        .map_err(|e| ErrorKind::Unexpected(e.to_string()))
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
