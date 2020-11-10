extern crate apex_rs;
extern crate dotenv;
#[macro_use]
extern crate log;

use apex_rs::errors::ErrorKind;
use apex_rs::importing::events::MessageTiming;
use apex_rs::importing::kafka::import_kafka;
use apex_rs::reporting::prometheus::report_prometheus;
use dotenv::dotenv;
use tokio::sync::mpsc::*;

#[tokio::main]
async fn main() {
    env_logger::init();
    debug!(target: "apex", "Booting");
    if cfg!(debug_assertions) {
        match dotenv() {
            Ok(_) => info!(target: "apex", "Initialized .env"),
            Err(e) => warn!(target: "apex", "Error loading .env: {}", e),
        }
    }

    let (mut tx, mut rx) = channel::<Result<MessageTiming, ErrorKind>>(100);

    tokio::try_join!(import_kafka(&mut tx), report_prometheus(&mut rx)).unwrap();
}
