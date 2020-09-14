use crate::app_config::AppConfig;
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
use uuid::Uuid;

fn get_app_config() -> AppConfig {
    AppConfig::default()
}

fn secret_for_print(v: Option<&String>) -> isize {
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
    data_server_url: '{}'
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
            secret_for_print(cfg.client_secret.as_ref()),
            cfg.data_server_timeout,
            cfg.data_server_url,
            cfg.disable_persistence,
            cfg.enable_unsafe_methods,
            secret_for_print(cfg.jwt_encryption_token.as_ref()),
            cfg.port,
            cfg.redis_url,
            secret_for_print(cfg.service_guest_token.as_ref()),
            value_for_print(cfg.session_cookie_name.clone()),
            value_for_print(cfg.session_cookie_sig_name.clone()),
            secret_for_print(cfg.session_secret.as_ref()),
    );
}

pub async fn serve() -> std::io::Result<()> {
    let config = AppConfig::default();
    if cfg!(debug_assertions) {
        print_config(&config);
    }
    let pool = DbContext::default_pool(&config.database_url);
    let address = format!("{}:{}", config.binding, config.port);

    HttpServer::new(move || {
        let mut app = App::new()
            .data(config.clone())
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

        if config.enable_unsafe_methods {
            app = app.service(update);
        }

        app
    })
    .bind(address)?
    .run()
    .await
}
