use crate::db::db_context::DbContext;
use crate::serving::assets::favicon;
use crate::serving::bulk::bulk;
use crate::serving::hpf::{hpf, tpf};
use crate::serving::service_info::service_info;
use crate::serving::show_resource::{random_resource, show_resource, show_resource_ext};
use crate::serving::update::update;
use actix_web::{middleware, App, HttpServer};
use std::env;

pub async fn serve() -> std::io::Result<()> {
    let pool = DbContext::default_pool();

    HttpServer::new(move || {
        let mut app = App::new()
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .service(favicon)
            .service(bulk)
            .service(service_info)
            .service(tpf)
            .service(hpf)
            .service(random_resource)
            .service(show_resource_ext)
            .service(show_resource);

        let enable_unsafe =
            env::var("ENABLE_UNSAFE_METHODS").unwrap_or_else(|_| String::from("false"));
        if enable_unsafe == "true" {
            app = app.service(update);
        }

        app
    })
    .bind("0.0.0.0:3030")?
    .run()
    .await
}
