use crate::db::db_context::DbContext;
use crate::serving::assets::favicon;
use crate::serving::bulk::bulk;
use crate::serving::welcome::welcome;
use crate::serving::show_resource::{random_resource, show_resource, show_resource_ext};
use actix_web::{middleware, App, HttpServer};
use std::{env};

pub async fn serve() -> std::io::Result<()> {
    let pool = DbContext::default_pool();
    let address_env = env::var("SERVER_ADDRESS");
    let address = match address_env {
        Ok(address_env) => { address_env },
        Err(_e) => { String::from("0.0.0.0:8080") },
    };

    println!("Listening at http://{}", address);

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .service(favicon)
            .service(bulk)
            .service(random_resource)
            .service(show_resource_ext)
            .service(show_resource)
            .service(welcome)
    })
    .bind(address)?
    .run()
    .await
}
