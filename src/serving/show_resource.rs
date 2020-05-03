use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::{doc_by_id, random_doc};
use crate::hashtuple::LookupTable;
use crate::serving::serialization::hash_model_to_response;
use actix_web::{get, web, Responder};

#[get("/random")]
pub(crate) async fn random_resource<'a>(pool: web::Data<DbPool>) -> impl Responder {
    let pl = pool.into_inner();
    let mut lookup_table = LookupTable::new();

    let random_doc = web::block(move || {
        let ctx = DbContext::new(&pl);

        match random_doc(&ctx, &mut lookup_table) {
            Some(model) => Ok((model, lookup_table)),
            None => Err(404),
        }
    })
    .await;

    hash_model_to_response(random_doc)
}

#[get("/{id}")]
pub(crate) async fn show_resource<'a>(
    pool: web::Data<DbPool>,
    info: web::Path<(i32,)>,
) -> impl Responder {
    let id = info.0;
    let pl = pool.into_inner();
    let mut lookup_table = LookupTable::new();

    let doc = web::block(move || {
        let ctx = DbContext::new(&pl);

        match doc_by_id(&ctx, &mut lookup_table, id as i64) {
            Some(model) => Ok((model, lookup_table)),
            None => Err(404),
        }
    })
    .await;

    hash_model_to_response(doc)
}
