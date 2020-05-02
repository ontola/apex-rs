use crate::db::db_context::DbContext;
use crate::serving::show_resource::show_resource;
use actix_web::{middleware, App, HttpServer};

pub async fn serve() -> std::io::Result<()> {
    let pool = DbContext::default_pool();

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .service(show_resource)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
