use crate::app_config::AppConfig;
use crate::db::db_context::DbContext;
use crate::serving::assets::favicon;
use crate::serving::bulk::bulk;
use crate::serving::health::health;
use crate::serving::hpf::{hpf, tpf};
use crate::serving::metrics::metrics;
use crate::serving::reporter::Reporter;
use crate::serving::service_info::service_info;
use crate::serving::show_resource::{random_resource, show_resource, show_resource_ext};
use crate::serving::update::update;
use actix_http::http::{HeaderName, HeaderValue};
use actix_web::dev::Service;
use actix_web::{middleware, App, HttpServer};
use futures::io::ErrorKind;
use uuid::Uuid;

fn secret_for_print(v: Option<String>) -> isize {
    v.map_or(-1 as isize, |v| v.len() as isize)
}

fn value_for_print(v: Option<String>) -> String {
    format!("'{}'", v.unwrap_or("[EMPTY]".into()))
}

fn print_config(cfg: &AppConfig) {
    debug!(target: "apex", "App config
    binding: '{}'
    client_id: {}
    client_secret: {}
    data_server_timeout: '{}'
    data_server_url: {}
    database_url: {}
    database_name: {}
    disable_persistence: '{}'
    enable_unsafe_methods: '{}'
    jwt_encryption_token: {}
    port: '{}'
    redis_url: '{}'
    service_guest_token: {}
    session_cookie_name: {}
    session_cookie_sig_name: {}
    session_secret: {}", 
            cfg.binding,
            value_for_print(cfg.client_id.clone()),
            secret_for_print(cfg.client_secret.clone()),
            cfg.data_server_timeout,
            value_for_print(cfg.data_server_url.clone()),
            secret_for_print(cfg.database_url.clone()),
            value_for_print(Some(cfg.database_name.clone())),
            cfg.disable_persistence,
            cfg.enable_unsafe_methods,
            secret_for_print(cfg.jwt_encryption_token.clone()),
            cfg.port,
            cfg.redis_url,
            secret_for_print(cfg.service_guest_token.clone()),
            value_for_print(cfg.session_cookie_name.clone()),
            value_for_print(cfg.session_cookie_sig_name.clone()),
            secret_for_print(cfg.session_secret.clone()),
    );
}

pub async fn serve() -> std::io::Result<()> {
    let config = AppConfig::default();
    if cfg!(debug_assertions) {
        print_config(&config);
    }
    let reporter = Reporter::default();
    let pool = DbContext::default_pool(config.database_url.clone(), config.database_pool_size)
        .map_err(|e| {
            error!(target: "apex", "{}", e);
            ErrorKind::Other
        })?;
    let address = format!("{}:{}", config.binding, config.port);

    HttpServer::new(move || {
        let app = App::new()
            .data(config.clone())
            .data(pool.clone())
            .data(reporter.clone())
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
            .service(metrics)
            .service(favicon)
            .service(bulk)
            .service(health)
            .service(service_info);

        let app = if !config.disable_persistence {
            app.service(tpf).service(hpf)
        } else {
            app
        };

        let mut app = app
            .service(random_resource)
            .service(show_resource_ext)
            .service(show_resource);

        if config.enable_unsafe_methods {
            app = app.service(update);
        }

        app
    })
    .bind(address)?
    .run()
    .await
}
