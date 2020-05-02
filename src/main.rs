#[macro_use]
extern crate log;
extern crate dotenv;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate dotenv_codegen;

mod db;
mod hashtuple;
mod importing;
mod reporting;

use crate::importing::events::MessageTiming;
use crate::importing::importer::import;
use crate::reporting::reporter::report;
use dotenv::dotenv;
use tokio::sync::mpsc::*;

#[tokio::main]
async fn main() {
    println!("Booting");
    dotenv().ok();
    println!("Initialized .env");

    let (mut tx, mut rx) = channel::<MessageTiming>(100);

    tokio::try_join!(import(&mut tx), report(&mut rx));
}
