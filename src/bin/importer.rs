extern crate apex_rs;
extern crate dotenv;
#[macro_use]
extern crate log;

use apex_rs::errors::ErrorKind;
use apex_rs::importing::events::MessageTiming;
use apex_rs::importing::importer::import;
use apex_rs::reporting::reporter::report;
use dotenv::dotenv;
use tokio::sync::mpsc::*;

#[tokio::main]
async fn main() {
    env_logger::init();
    debug!(target: "apex", "Booting");
    if cfg!(debug_assertions) {
        dotenv().ok();
        info!(target: "apex", "Initialized .env");
    }

    let (mut tx, mut rx) = channel::<Result<MessageTiming, ErrorKind>>(100);

    tokio::try_join!(import(&mut tx), report(&mut rx)).unwrap();
}
