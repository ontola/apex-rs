use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::doc_by_id;
use crate::hashtuple::{HashModel, LookupTable, Statement};
use crate::serving::response_type::{ResponseType, NQUADS_MIME, NTRIPLES_MIME};
use crate::serving::responses::set_default_headers;
use crate::serving::serialization::{
    bulk_result_to_hextuples, bulk_result_to_nquads, bulk_result_to_ntriples, BulkInput,
};
use actix_web::http::header;
use actix_web::{post, web, HttpResponse, Responder};
use futures::StreamExt;
use percent_encoding::percent_decode_str;
use serde_derive::Deserialize;

#[derive(Deserialize)]
pub(crate) struct FormData {
    resource: Vec<String>,
}

#[post("/link-lib/bulk")]
pub(crate) async fn bulk<'a>(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    mut payload: web::Payload,
) -> impl Responder {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = payload.next().await {
        bytes.extend_from_slice(&item.unwrap());
    }
    let byte_vec = bytes.to_vec();

    let body = std::str::from_utf8(&byte_vec.as_slice()).unwrap();
    println!("body: {}", body);

    let pl = pool.into_inner();
    let mut lookup_table = LookupTable::default();

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

    let bulk_docs = web::block(move || -> Result<BulkInput, i32> {
        let ctx = DbContext::new(&pl);
        let models: Vec<Option<HashModel>> = resources
            .iter()
            .map(|r| {
                percent_decode_str(r)
                    .decode_utf8()
                    .unwrap()
                    .split('/')
                    .last()
                    .unwrap()
                    .parse::<i64>()
                    .unwrap()
            })
            .map(|id| {
                if let Some(mut doc) = doc_by_id(&ctx, &mut lookup_table, id) {
                    doc.push(status_code_statement(&mut lookup_table, id, 200));

                    Some(doc)
                } else {
                    Some(vec![status_code_statement(&mut lookup_table, id, 404)])
                }
            })
            .collect();

        Ok((models, lookup_table))
    })
    .await;

    if bulk_docs.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    let (body, response_type) = if let Some(accept) = req.headers().get(header::ACCEPT) {
        let accept = accept.to_str().unwrap();
        if accept == NQUADS_MIME {
            (
                bulk_result_to_nquads(bulk_docs.unwrap()),
                ResponseType::NQUADS,
            )
        } else if accept == NTRIPLES_MIME {
            (
                bulk_result_to_ntriples(bulk_docs.unwrap()),
                ResponseType::NTRIPLES,
            )
        } else {
            (
                bulk_result_to_hextuples(bulk_docs.unwrap()),
                ResponseType::HEXTUPLE,
            )
        }
    } else {
        (
            bulk_result_to_hextuples(bulk_docs.unwrap()),
            ResponseType::HEXTUPLE,
        )
    };

    set_default_headers(&mut HttpResponse::Ok(), &response_type).body(body)
}

fn status_code_statement(lookup_table: &mut LookupTable, id: i64, status: i16) -> Statement {
    Statement {
        subject: lookup_table
            .ensure_value(format!("https://id.openraadsinformatie.nl/{}", id).as_str()),
        predicate: lookup_table.ensure_value("http://www.w3.org/2011/http#statusCode"),
        value: lookup_table.ensure_value(status.to_string().as_str()),
        datatype: lookup_table.ensure_value("http://www.w3.org/2001/XMLSchema#integer"),
        language: lookup_table.ensure_value(""),
        graph: lookup_table.ensure_value("http://purl.org/link-lib/meta"),
    }
}
