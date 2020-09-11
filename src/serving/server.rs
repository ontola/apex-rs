use crate::db::db_context::DbContext;
use crate::serving::assets::favicon;
use crate::serving::bulk::bulk;
use crate::serving::health::health;
use crate::serving::hpf::{hpf, tpf};
use crate::serving::service_info::service_info;
use crate::serving::show_resource::{random_resource, show_resource, show_resource_ext};
use crate::serving::update::update;
use actix_http::http::{HeaderName, HeaderValue};
use actix_web::dev::Service;
use actix_web::{middleware, App, HttpServer};
use std::env;
use uuid::Uuid;

pub async fn serve() -> std::io::Result<()> {
    let pool = DbContext::default_pool();
    let binding = env::var("BINDING").unwrap_or("0.0.0.0".into());
    let port = env::var("PORT").unwrap_or("3030".into());
    let address = format!("{}:{}", binding, port);

    HttpServer::new(move || {
        let mut app = App::new()
            .data(pool.clone())
            .wrap(middleware::Logger::new(
                r#"[%{X-Request-Id}i] %a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#,
            ))
            .wrap_fn(|mut req, srv| {
                let req = match req.headers().get("X-Request-Id") {
                    Some(_) => req,
                    None => {
                        let id = Uuid::new_v4().to_hyphenated().to_string();

                        req.headers_mut().insert(
                            HeaderName::from_static("x-request-id"),
                            HeaderValue::from_str(id.as_str()).unwrap(),
                        );
                        req
                    }
                };

                let fut = srv.call(req);
                async {
                    let res = fut.await?;
                    Ok(res)
                }
            })
            .wrap(middleware::Compress::default())
            .service(favicon)
            .service(bulk)
            .service(health)
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
    .bind(address)?
    .run()
    .await
}
