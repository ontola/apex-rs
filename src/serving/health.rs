use crate::app_config::AppConfig;
use crate::db::db_context::{DbContext, DbPool};
use crate::serving::response_type::ResponseType::JSON;
use crate::serving::responses::set_default_headers;
use crate::serving::ua::basic_ua;
use actix_web::{get, web, HttpResponse, Responder};
use serde::Serialize;

#[derive(Serialize)]
struct Status {
    name: String,
    database_name: Option<String>,
    est_documents: i64,
}

#[get("/link-lib/d/health")]
pub(crate) async fn health<'a>(pool: web::Data<DbPool>) -> impl Responder {
    let ctx = DbContext::new(&pool);
    let name = basic_ua();

    let counts = ctx.est_counts();
    let db = AppConfig::default();
    let database_name = if cfg!(debug_assertions) {
        Some(db.database_name)
    } else {
        None
    };

    let status = Status {
        name,
        database_name,
        est_documents: counts.documents,
    };

    set_default_headers(&mut HttpResponse::Ok(), &JSON).json(status)
}
