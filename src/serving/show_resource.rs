use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::{doc_by_iri, random_doc};
use crate::errors::ErrorKind;
use crate::serving::response_type::ResponseType;
use crate::serving::responses::set_default_headers;
use crate::serving::serialization::{
    hash_model_to_hextuples, hash_model_to_ntriples, hash_model_to_turtle,
};
use actix_web::error::BlockingError;
use actix_web::http::{header, HeaderMap};
use actix_web::{get, web, HttpResponse, Responder};
use std::str::FromStr;
use std::sync::Arc;

#[get("/random")]
pub(crate) async fn random_resource<'a>(pool: web::Data<DbPool>) -> impl Responder {
    let pl = pool.into_inner();

    let random_doc = web::block(move || {
        let mut ctx = DbContext::new(&pl);

        random_doc(&mut ctx).map(|(_, model)| (model, ctx.lookup_table))
    })
    .await;

    match random_doc {
        Ok(doc) => HttpResponse::Ok().body(hash_model_to_hextuples((doc.0, &doc.1))),
        Err(BlockingError::Error(ErrorKind::EmptyDocument)) => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!(target: "apex", "Unknown error: {}", e);

            HttpResponse::ServiceUnavailable().finish()
        }
    }
}

#[get("/{id}.{ext}")]
pub(crate) async fn show_resource_ext<'a>(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    info: web::Path<(String, String)>,
) -> HttpResponse {
    if let Ok(response_type) = ResponseType::from_ext(&info.1) {
        let path = &info.0;
        let pl = pool.into_inner();

        match iri_from_request(req, &path) {
            Some(iri) => show(pl, &iri, response_type).await,
            None => HttpResponse::BadRequest().finish(),
        }
    } else {
        HttpResponse::NotAcceptable().finish()
    }
}

#[get("/{id}")]
pub(crate) async fn show_resource<'a>(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    info: web::Path<(String,)>,
) -> HttpResponse {
    let response_type = match negotiate(req.headers(), &None) {
        Some(s) => s,
        None => return HttpResponse::NotAcceptable().finish(),
    };
    let path = &info.0;
    let pl = pool.into_inner();

    match iri_from_request(req, &path) {
        Some(iri) => show(pl, &iri, response_type).await,
        None => HttpResponse::BadRequest().finish(),
    }
}

#[allow(clippy::borrow_interior_mutable_const)]
async fn show<'a>(pl: Arc<DbPool>, iri: &str, response_type: ResponseType) -> HttpResponse {
    let iri_move = String::from(iri);

    let doc = web::block(move || {
        let mut ctx = DbContext::new(&pl);

        match doc_by_iri(&mut ctx, &iri_move) {
            Ok((_, model)) => Ok((model, ctx.lookup_table)),
            Err(_) => Err(404),
        }
    })
    .await;

    if doc.is_err() {
        return HttpResponse::NotFound().finish();
    }

    let (model, lookup_table) = doc.unwrap();
    let serialization = match response_type {
        ResponseType::HEXTUPLE => hash_model_to_hextuples((model, &lookup_table)),
        ResponseType::NTRIPLES | ResponseType::NQUADS => {
            hash_model_to_ntriples((model, &lookup_table))
        }
        ResponseType::TURTLE => hash_model_to_turtle((model, &lookup_table)),
    };

    set_default_headers(&mut HttpResponse::Ok(), &response_type)
        .set(header::CacheControl(vec![
            header::CacheDirective::MaxAge(86400u32),
            header::CacheDirective::Public,
        ]))
        .set_header(
            "Content-Disposition",
            format!("inline; filename={}", iri_to_filename(&iri, &response_type)),
        )
        .body(serialization)
}

fn iri_from_request(req: actix_web::HttpRequest, path: &str) -> Option<String> {
    let host = match req.headers().get("Host")?.to_str() {
        Ok(v) => v,
        Err(_) => return None,
    };

    Some(format!("https://{}/{}", host, path))
}

fn iri_to_filename(iri: &str, response_type: &ResponseType) -> String {
    format!(
        "{}.{}",
        iri.split('/').last().unwrap(),
        response_type.to_ext()
    )
}

fn negotiate(headers: &HeaderMap, ext: &Option<String>) -> Option<ResponseType> {
    if ext.is_some() {
        let extention = ext.as_ref().unwrap();

        return if let Ok(response_type) = ResponseType::from_ext(extention) {
            Some(response_type)
        } else {
            None
        };
    }

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
