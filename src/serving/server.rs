use crate::db::db_context::DbContext;
<<<<<<< HEAD
use crate::serving::show_resource::show_resource;
=======
use crate::serving::bulk::bulk;
use crate::serving::show_resource::{random_resource, show_resource};
>>>>>>> 291fbcf... fixup! WIP: Bulk api
use actix_web::{middleware, App, HttpServer};

pub async fn serve() -> std::io::Result<()> {
    let pool = DbContext::default_pool();

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .service(bulk)
            .service(show_resource)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
