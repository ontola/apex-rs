use actix_web::{get, HttpResponse};
use http::status::StatusCode;
use std::fs::read_to_string;

const CUSTOM_PATH: &str = "./static_custom/index.html";
const FALLBACK_PATH: &str = "./static/index.html";

#[get("/")]
pub(crate) async fn welcome() -> HttpResponse {
    let file = read_to_string(CUSTOM_PATH)
        .or(read_to_string(FALLBACK_PATH));

    let mut builder = HttpResponse::build(StatusCode::OK);

    match file {
        Ok(st) => builder.body(st),
        Err(e) => builder.body(format!("{}.\nNo static page found. Add one in {} or {}.", e, CUSTOM_PATH, FALLBACK_PATH)),
    }
}
