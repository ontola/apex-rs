use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::doc_by_iri;
use crate::errors::ErrorKind;
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
use std::collections::HashSet;

#[derive(Deserialize)]
pub(crate) struct FormData {
    resource: Vec<String>,
}

#[post("/link-lib/bulk")]
pub(crate) async fn bulk<'a>(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    payload: web::Payload,
) -> impl Responder {
    let pl = pool.into_inner();
    let mut lookup_table = LookupTable::default();
    let resources = resources_from_payload(payload).await;
    if resources.is_err() {
        return HttpResponse::BadRequest().finish();
    }

    let bulk_docs = web::block(move || -> Result<BulkInput, i32> {
        let ctx = DbContext::new(&pl);
        let models: Vec<Option<HashModel>> = resources
            .unwrap()
            .iter()
            .map(|r| percent_decode_str(r).decode_utf8().unwrap())
            .map(|iri| {
                if let Some(doc) = doc_by_iri(&ctx, &mut lookup_table, &iri) {
                    let mut model = doc.1;
                    model.push(status_code_statement(&mut lookup_table, &iri, 200));

                    Some(model)
                } else {
                    Some(vec![status_code_statement(&mut lookup_table, &iri, 404)])
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

fn status_code_statement(lookup_table: &mut LookupTable, iri: &str, status: i16) -> Statement {
    Statement {
        subject: lookup_table.ensure_value(iri),
        predicate: lookup_table.ensure_value("http://www.w3.org/2011/http#statusCode"),
        value: lookup_table.ensure_value(status.to_string().as_str()),
        datatype: lookup_table.ensure_value("http://www.w3.org/2001/XMLSchema#integer"),
        language: lookup_table.ensure_value(""),
        graph: lookup_table.ensure_value("http://purl.org/link-lib/meta"),
    }
}

async fn resources_from_payload(mut payload: web::Payload) -> Result<HashSet<String>, ErrorKind> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = payload.next().await {
        bytes.extend_from_slice(&item.unwrap());
    }
    let byte_vec = bytes.to_vec();

    let body = std::str::from_utf8(&byte_vec.as_slice()).unwrap();

    let resources = serde_qs::from_str::<FormData>(
        &body
            .replace("%5B", "[") // <= quick and dirty percent decode
            .replace("%5D", "]"),
    );

    match resources {
        Ok(resources) => {
            let t = resources
                .resource
                .iter()
                .map(|c| c.to_string())
                .collect::<HashSet<String>>();

            Ok(t)
        }
        Err(_) => Err(ErrorKind::Msg(String::from("Bad payload"))),
    }
}
