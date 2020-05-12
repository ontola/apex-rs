use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::{doc_by_id, random_doc};
use crate::hashtuple::LookupTable;
use crate::serving::response_type::ResponseType;
use crate::serving::serialization::{
    hash_model_to_hextuples, hash_model_to_ntriples, hash_model_to_turtle,
};
use actix_web::http::{header, HeaderMap};
use actix_web::{get, web, HttpResponse, Responder};
use std::str::FromStr;

#[get("/random")]
pub(crate) async fn random_resource<'a>(pool: web::Data<DbPool>) -> impl Responder {
    let pl = pool.into_inner();
    let mut lookup_table = LookupTable::default();

    let random_doc = web::block(move || {
        let ctx = DbContext::new(&pl);

        match random_doc(&ctx, &mut lookup_table) {
            Some(model) => Ok((model, lookup_table)),
            None => Err(404),
        }
    })
    .await;

    if random_doc.is_err() {
        return HttpResponse::NotFound().finish();
    }

    HttpResponse::Ok().body(hash_model_to_hextuples(random_doc.unwrap()))
}

#[get("/{id}")]
#[allow(clippy::borrow_interior_mutable_const)]
pub(crate) async fn show_resource<'a>(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    info: web::Path<(i32,)>,
) -> HttpResponse {
    let response_type = match negotiate(req.headers()) {
        Some(s) => s,
        None => return HttpResponse::NotAcceptable().finish(),
    };
    let id = info.0;
    let pl = pool.into_inner();
    let mut lookup_table = LookupTable::default();

    let doc = web::block(move || {
        let ctx = DbContext::new(&pl);

        match doc_by_id(&ctx, &mut lookup_table, id as i64) {
            Some(model) => Ok((model, lookup_table)),
            None => Err(404),
        }
    })
    .await;

    if doc.is_err() {
        return HttpResponse::NotFound().finish();
    }

    let serialization = match response_type {
        ResponseType::HEXTUPLE => hash_model_to_hextuples(doc.unwrap()),
        ResponseType::NTRIPLES | ResponseType::NQUADS => hash_model_to_ntriples(doc.unwrap()),
        ResponseType::TURTLE => hash_model_to_turtle(doc.unwrap()),
    };

    HttpResponse::Ok()
        .set(header::CacheControl(vec![
            header::CacheDirective::MaxAge(86400u32),
            header::CacheDirective::Public,
        ]))
        .set_header(header::CONTENT_TYPE, response_type.to_string())
        .set_header(header::VARY, header::CONTENT_TYPE.to_string())
        .body(serialization)
}

fn negotiate(headers: &HeaderMap) -> Option<ResponseType> {
    match headers.get(header::ACCEPT) {
        None => None,
        Some(h) => match h.to_str() {
            Err(_) => None,
            Ok(accept) => {
                if accept.is_empty() || accept.contains("*/*") {
                    Some(ResponseType::HEXTUPLE)
                } else if let Ok(m) = ResponseType::from_str(accept) {
                    Some(m)
                } else {
                    None
                }
            }
        },
    }
}
