use crate::db::db_context::DbContext;
use crate::serving::assets::favicon;
use crate::serving::bulk::bulk;
use crate::serving::hpf::{hpf, tpf};
use crate::serving::service_info::service_info;
use crate::serving::show_resource::{random_resource, show_resource, show_resource_ext};
use actix_web::{middleware, App, HttpServer};

pub async fn serve() -> std::io::Result<()> {
    let pool = DbContext::default_pool();

    HttpServer::new(move || {
        App::new()
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
            .service(show_resource)
    })
    .bind("0.0.0.0:3030")?
    .run()
    .await
}
