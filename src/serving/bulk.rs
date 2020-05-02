use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::doc_by_id;
use crate::hashtuple::{HashModel, LookupTable};
use crate::serving::serialization::{bulk_result_to_response, BulkInput};
use actix_web::{post, web, Responder};
use futures::StreamExt;
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub(crate) struct FormData {
    resource: Vec<String>,
}

#[post("/link-lib/bulk")]
pub(crate) async fn bulk<'a>(pool: web::Data<DbPool>, mut payload: web::Payload) -> impl Responder {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = payload.next().await {
        bytes.extend_from_slice(&item.unwrap());
    }
    let byte_vec = bytes.to_vec();

    let body = std::str::from_utf8(&byte_vec.as_slice()).unwrap();
    println!("body: {}", body);

    let pl = pool.into_inner();
    let mut lookup_table = LookupTable::new();

    let resources = serde_qs::from_str::<FormData>(
        &body
            .replace("%5B", "[") // <= quick and dirty percent decode
            .replace("%5D", "]"),
    ) // <= quick and dirty percent decode
    .unwrap()
    .resource
    .iter()
    .map(|c| c.to_string())
    .collect::<Vec<String>>();

    let random_doc = web::block(move || {
        let ctx = DbContext::new(&pl);
        let models: Vec<Option<HashModel>> = resources
            .iter()
            .map(|r| r.split('/').last().unwrap().parse::<i64>().unwrap())
            .map(|id| doc_by_id(&ctx, &mut lookup_table, id))
            .collect();

        Ok((models, lookup_table))
    })
    .await;

    bulk_result_to_response(random_doc)
}
