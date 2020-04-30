#[macro_use]
extern crate log;
extern crate dotenv;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate dotenv_codegen;

mod db_context;
mod delta_processor;
mod document;
mod events;
mod hashtuple;
mod importer;
mod models;
mod parsing;
mod properties;
mod resources;
mod schema;

use crate::events::MessageTiming;
use crate::importer::*;
use dotenv::dotenv;
use tokio::sync::mpsc::*;

#[tokio::main]
async fn main() {
    println!("Booting");
    dotenv().ok();
    println!("Initialized .env");

    let (mut tx, mut rx) = channel::<MessageTiming>(100);

    import(&mut tx).await.unwrap();
}
