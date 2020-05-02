use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::doc_by_id;
use crate::hashtuple::LookupTable;
use crate::serving::serialization::hash_model_to_response;
use actix_web::{get, web, HttpServer, Responder};

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
