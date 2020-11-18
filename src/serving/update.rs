use crate::db::db_context::{DbContext, DbPool};
use crate::errors::ErrorKind;
use crate::hashtuple::LookupTable;
use crate::importing::importer::process_message;
use crate::importing::parsing::{parse_hndjson, DocumentSet};
use crate::serving::response_type::ResponseType;
use crate::serving::responses::set_default_headers;
use actix_web::{post, web, HttpResponse, Responder};
use futures::StreamExt;

#[post("/update")]
pub(crate) async fn update<'a>(pool: web::Data<DbPool>, payload: web::Payload) -> impl Responder {
    let mut ctx = DbContext::new(&pool);
    let delta = match parse_payload(&mut ctx.lookup_table, payload).await {
        Ok(delta) => delta,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let total: usize = delta.iter().map(|(_, ds)| ds.len()).sum();
    debug!(target: "apex", "Recieved {} statements from body", total);
    let mut res = match process_message(&mut ctx, delta).await {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => {
            warn!(target: "apex", "Processing delta message failed: {}", e);
            HttpResponse::InternalServerError()
        }
    };

    set_default_headers(&mut res, &ResponseType::HEXTUPLE).finish()
}

async fn parse_payload(
    mut lookup_table: &mut LookupTable,
    mut payload: web::Payload,
) -> Result<DocumentSet, ErrorKind> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = payload.next().await {
        bytes.extend_from_slice(&item.map_err(|e| ErrorKind::Unexpected(e.to_string()))?);
    }

    parse_hndjson(&mut lookup_table, bytes.as_ref())
}
