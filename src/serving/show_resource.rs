use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::{doc_by_id, random_doc};
use crate::hashtuple::LookupTable;
use crate::serving::response_type::ResponseType;
use crate::serving::responses::set_default_headers;
use crate::serving::serialization::{
    hash_model_to_hextuples, hash_model_to_ntriples, hash_model_to_turtle,
};
use actix_web::http::{header, HeaderMap};
use actix_web::{get, web, HttpResponse, Responder};
use std::str::FromStr;
use std::sync::Arc;

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

    let doc = random_doc.unwrap();
    HttpResponse::Ok().body(hash_model_to_hextuples((doc.0, &doc.1)))
}

#[get("/{id}.{ext}")]
pub(crate) async fn show_resource_ext<'a>(
    pool: web::Data<DbPool>,
    info: web::Path<(i32, String)>,
) -> HttpResponse {
    if let Ok(response_type) = ResponseType::from_ext(&info.1) {
        let id = info.0;
        let pl = pool.into_inner();
        show(pl, id, response_type).await
    } else {
        HttpResponse::NotAcceptable().finish()
    }
}

#[get("/{id}")]
pub(crate) async fn show_resource<'a>(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    info: web::Path<(i32,)>,
) -> HttpResponse {
    let response_type = match negotiate(req.headers(), &None) {
        Some(s) => s,
        None => return HttpResponse::NotAcceptable().finish(),
    };
    let id = info.0;
    let pl = pool.into_inner();
    show(pl, id, response_type).await
}

#[allow(clippy::borrow_interior_mutable_const)]
async fn show(pl: Arc<DbPool>, id: i32, response_type: ResponseType) -> HttpResponse {
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

    let doc = doc.unwrap();
    let serialization = match response_type {
        ResponseType::HEXTUPLE => hash_model_to_hextuples((doc.0, &doc.1)),
        ResponseType::NTRIPLES | ResponseType::NQUADS => hash_model_to_ntriples((doc.0, &doc.1)),
        ResponseType::TURTLE => hash_model_to_turtle((doc.0, &doc.1)),
    };

    set_default_headers(&mut HttpResponse::Ok(), &response_type)
        .set(header::CacheControl(vec![
            header::CacheDirective::MaxAge(86400u32),
            header::CacheDirective::Public,
        ]))
        .set_header(
            "Content-Disposition",
            format!("inline; filename={}.{}", id, response_type.to_ext()),
        )
        .body(serialization)
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
