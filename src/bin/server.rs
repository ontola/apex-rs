extern crate apex_rs;
#[macro_use]
extern crate log;

use apex_rs::serving::serve;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "app=info,actix_web=info,diesel=debug");
    env_logger::init();

    serve().await
}
