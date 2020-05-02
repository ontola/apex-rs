use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::doc_by_id;
use crate::hashtuple::LookupTable;
use crate::serving::serialization::{hash_to_hex, ND_DELIMITER};
use actix_web::{get, web, HttpResponse, HttpServer, Responder};

#[get("/{id}")]
pub(crate) async fn show_resource<'a>(
    pool: web::Data<DbPool>,
    info: web::Path<(i32,)>,
) -> impl Responder {
    let id = info.0;
    let pl = pool.into_inner();
    let mut lookup_table = LookupTable::new();

    let test = web::block(move || {
        let ctx = DbContext::new(&pl);

        match doc_by_id(&ctx, &mut lookup_table, id as i64) {
            Some(model) => Ok((model, lookup_table)),
            None => Err(404),
        }
    })
    .await;

    match test {
        Err(_code) => HttpResponse::NotFound().finish(),
        Ok((doc, filled_table)) => {
            let mut output = Vec::new();

            let test = hash_to_hex(doc, &filled_table);
            for h in test {
                output.append(serde_json::to_vec(&h).unwrap().as_mut());
                output.push(ND_DELIMITER);
            }

            HttpResponse::Ok()
                // .content_type("application/hex+ndjson")
                .body(output)
        }
    }
}
