extern crate apex_rs;
extern crate dotenv;
#[macro_use]
extern crate dotenv_codegen;
#[macro_use]
extern crate log;

use apex_rs::serving::serve;
use dotenv::dotenv;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "app=debug,actix_web=info,diesel=debug");
    env_logger::init();
    if cfg!(debug_assertions) {
        dotenv().ok();
        println!("Initialized .env");
    }

    serve().await
}
