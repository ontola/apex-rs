extern crate apex_rs;
extern crate dotenv;
#[macro_use]
extern crate dotenv_codegen;

use apex_rs::importing::events::MessageTiming;
use apex_rs::importing::importer::import;
use apex_rs::reporting::reporter::report;
use dotenv::dotenv;
use tokio::sync::mpsc::*;

#[tokio::main]
async fn main() {
    println!("Booting");
    if cfg!(debug_assertions) {
        dotenv().ok();
        println!("Initialized .env");
    }

    let (mut tx, mut rx) = channel::<MessageTiming>(100);

    tokio::try_join!(import(&mut tx), report(&mut rx));
}
