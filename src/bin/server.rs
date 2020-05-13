extern crate apex_rs;
extern crate dotenv;
#[macro_use]
extern crate log;

use apex_rs::serving::serve;
use dotenv::dotenv;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    info!(target: "apex", "Booting");
    env_logger::init();
    if cfg!(debug_assertions) {
        dotenv().ok();
        info!(target: "apex", "Initialized .env");
    }

    serve().await
}
