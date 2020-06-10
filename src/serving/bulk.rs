use crate::db::cache_control::CacheControl;
use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::doc_by_iri;
use crate::errors::ErrorKind;
use crate::hashtuple::{HashModel, LookupTable, Statement};
use crate::importing::importer::process_message;
use crate::importing::parsing::{parse_hndjson, DocumentSet};
use crate::rdf::iri_utils::stem_iri;
use crate::serving::response_type::{ResponseType, NQUADS_MIME, NTRIPLES_MIME};
use crate::serving::responses::set_default_headers;
use crate::serving::serialization::{
    bulk_result_to_hextuples, bulk_result_to_nquads, bulk_result_to_ntriples,
};
use actix_web::client::Client;
use actix_web::http::header;
use actix_web::{post, web, HttpResponse, Responder};
use futures::StreamExt;
use percent_encoding::percent_decode_str;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;

#[derive(Deserialize, Serialize)]
pub(crate) struct FormData {
    resource: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct SPIResourceRequestItem {
    iri: String,
    include: bool,
}

#[derive(Serialize)]
pub(crate) struct SPIBulkRequest<'a> {
    resources: &'a Vec<SPIResourceRequestItem>,
}

#[derive(Serialize)]
pub(crate) struct SPITenantFinderRequest {
    iri: String,
}

#[derive(Deserialize)]
pub(crate) struct SPITenantFinderResponse {
    database_schema: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct SPIResourceResponseItem {
    iri: String,
    status: i16,
    cache: CacheControl,
    body: Option<String>,
}

pub(crate) struct Resource {
    iri: String,
    status: i16,
    cache_control: CacheControl,
    data: HashModel,
}

#[post("/link-lib/bulk")]
pub(crate) async fn bulk<'a>(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    payload: web::Payload,
) -> impl Responder {
    let pl = pool.clone().into_inner();
    let resource_set = resources_from_payload(payload).await;
    if resource_set.is_err() {
        return HttpResponse::BadRequest().finish();
    };
    let resource_set = resource_set.unwrap();
    let resources: Vec<String> = resource_set
        .into_iter()
        .map(|r| String::from(percent_decode_str(&r).decode_utf8().unwrap()))
        .collect();

    debug!(target: "apex", "Requested {} resources", resources.len());
    let bulk_resources = resources.clone();

    let bulk_docs = web::block(move || -> Result<(Vec<Resource>, LookupTable), i32> {
        let mut ctx = DbContext::new(&pl);
        let models: Vec<Resource> = bulk_resources
            .into_iter()
            .map(stem_iri)
            .map(|iri| {
                if let Ok(doc) = doc_by_iri(&mut ctx, &iri) {
                    debug!(
                        "Load ok: {}, cc: {}, stmts: {}",
                        doc.0.iri,
                        CacheControl::from(doc.0.cache_control),
                        doc.1.len()
                    );
                    Resource {
                        iri,
                        status: if doc.1.is_empty() { 204 } else { 200 },
                        cache_control: doc.0.cache_control.into(),
                        data: doc.1,
                    }
                } else {
                    debug!("Load failed: {}", iri);
                    Resource {
                        iri,
                        status: 404,
                        cache_control: CacheControl::Private,
                        data: HashModel::new(),
                    }
                }
            })
            .collect();

        Ok((models, ctx.lookup_table))
    })
    .await;

    if bulk_docs.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    let (mut bulk_docs, mut lookup_table) = bulk_docs.unwrap();
    let resources_in_cache: Vec<&str> = bulk_docs
        .iter()
        .filter(|r| r.status == 200)
        .map(|r| r.iri.as_str())
        .collect();
    debug!("resources_in_cache {}", resources_in_cache.join(","));
    let complete_public = bulk_docs
        .iter()
        .filter(|r| {
            debug!("cached resource {} is {}", r.iri, r.cache_control);
            r.cache_control == CacheControl::Public
        })
        .for_each(|r| debug!("public {}", r.iri));

    // 4. RS sends bulk authorize request to BE for all non-public resources (process The status code and cache headers per resource)
    let non_public_resources: Vec<String> = resources
        .into_iter()
        .filter(|iri| {
            !bulk_docs.iter().any(|r| {
                r.cache_control == CacheControl::Public && r.iri.as_str() == stem_iri(iri).as_str()
            })
        })
        .collect();

    if non_public_resources.len() > 0 {
        debug!("Authorize {} documents", non_public_resources.len());
        let auth_result =
            match authorize_resources(&req, non_public_resources, &resources_in_cache).await {
                Ok(data) => data,
                Err(err) => {
                    error!(target: "apex", "Unexpected error while authorizing: {}", err);
                    return HttpResponse::InternalServerError().finish();
                }
            };

        // 7. RS saves resources with cache headers to db according to policy
        let uncached_and_included: Vec<&SPIResourceResponseItem> = auth_result
            .iter()
            .filter(|r| {
                debug!(target: "apex", "Auth result; iri: {}, status: {}, cache: {}, included: {}", r.iri, r.status, r.cache, r.body.is_some());
                r.status == 200 && !resources_in_cache.contains(&r.iri.as_str()) && r.body.is_some()
            })
            .collect();

        let mut uncached_and_included_documents = vec![];

        for r in &uncached_and_included {
            let body = r.body.as_ref().unwrap();
            match parse_hndjson(&mut lookup_table, body.as_ref()) {
                Ok(data) => {
                    uncached_and_included_documents.push(data);
                }
                Err(e) => {
                    debug!(target: "apex", "Error while processing bulk request {}", e);

                    return HttpResponse::InternalServerError().finish();
                }
            }
        }

        let uncached_and_cacheable: Vec<DocumentSet> = uncached_and_included
            .iter()
            .enumerate()
            .filter(|(_, r)| r.cache != CacheControl::Private)
            .map(|(n, _)| uncached_and_included_documents.get(n).unwrap().clone())
            .collect();

        if !uncached_and_cacheable.is_empty() {
            debug!(target: "apex", "Writing {} uncached resources to db", uncached_and_cacheable.len());
            let pl = pool.into_inner();
            let mut ctx = DbContext::new(&pl);
            ctx.lookup_table = lookup_table;

            for r in uncached_and_cacheable {
                if let Err(e) = process_message(&mut ctx, r.clone()).await {
                    error!(target: "apex", "Error writing resource to database: {}", e);
                    return HttpResponse::InternalServerError().finish();
                }
            }

            lookup_table = ctx.lookup_table
        }

        for (n, docset) in uncached_and_included_documents.iter().enumerate() {
            let document = uncached_and_included.get(n).unwrap();

            for (iri, data) in docset {
                bulk_docs.push(Resource {
                    iri: iri.clone(),
                    status: document.status,
                    cache_control: document.cache,
                    data: data.to_vec(),
                });
            }
        }
    } else {
        debug!("All resources are public");
    }

    // 8. RS sends response back to client
    let bulk_docs = bulk_docs
        .into_iter()
        .map(|resource| {
            if resource.status == 404 {
                Some(vec![status_code_statement(
                    &mut lookup_table,
                    &resource.iri,
                    404,
                )])
            } else {
                let mut model = resource.data;
                model.push(status_code_statement(
                    &mut lookup_table,
                    &resource.iri,
                    resource.status,
                ));

                Some(model)
            }
        })
        .collect();

    let bulk_docs = (bulk_docs, lookup_table);

    let (body, response_type) = if let Some(accept) = req.headers().get(header::ACCEPT) {
        let accept = accept.to_str().unwrap();
        if accept == NQUADS_MIME {
            (bulk_result_to_nquads(bulk_docs), ResponseType::NQUADS)
        } else if accept == NTRIPLES_MIME {
            (bulk_result_to_ntriples(bulk_docs), ResponseType::NTRIPLES)
        } else {
            (bulk_result_to_hextuples(bulk_docs), ResponseType::HEXTUPLE)
        }
    } else {
        (bulk_result_to_hextuples(bulk_docs), ResponseType::HEXTUPLE)
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

async fn authorize_resources(
    req: &actix_web::HttpRequest,
    resources: Vec<String>,
    resources_in_cache: &Vec<&str>,
) -> Result<Vec<SPIResourceResponseItem>, ErrorKind> {
    let client = Client::default();
    let auth = req
        .headers()
        .get("authorization")
        .unwrap()
        .to_str()
        .unwrap();
    let website = req.headers().get("website-iri").unwrap().to_str().unwrap();
    let forward_for = req
        .headers()
        .get("x-forwarded-for")
        .unwrap()
        .to_str()
        .unwrap();
    let forward_host = req
        .headers()
        .get("x-forwarded-host")
        .unwrap()
        .to_str()
        .unwrap();

    let items = resources
        .iter()
        .map(stem_iri)
        .collect::<HashSet<String>>()
        .into_iter()
        .map(|iri| SPIResourceRequestItem {
            include: !resources_in_cache.contains(&iri.as_str()),
            iri,
        })
        .collect();
    let request = SPIBulkRequest { resources: &items };

    // Find tenant
    let core_api_url = env::var("ARGU_API_URL").unwrap();
    let tenant_req_body = SPITenantFinderRequest {
        iri: website.into(),
    };
    let tenant = client
        .get(format!("{}/_public/spi/find_tenant", core_api_url).as_str())
        .header(header::USER_AGENT, "Apex/1")
        .send_json(&tenant_req_body)
        .await
        .expect("Error finding tenant")
        .json::<SPITenantFinderResponse>()
        .await
        .expect("Error parsing tenant finder response");
    let tenant_path = tenant.database_schema;
    debug!(target: "apex", "Tenant: {}", tenant_path);

    // Create request builder and send request
    let response = client
        .post(format!("{}/{}/spi/bulk", core_api_url, tenant_path))
        .header(header::AUTHORIZATION, auth)
        .header(header::USER_AGENT, "Apex/1")
        .header("X-Forwarded-For", forward_for)
        .header("X-Forwarded-Host", forward_host)
        .header("X-Forwarded-Proto", "https")
        .header("X-Forwarded-Ssl", "on")
        .header("Website-IRI", website)
        .send_json(&request)
        .await;

    match response {
        Err(e) => {
            debug!(target: "apex", "Unexpected error sending bulk authorize: {}", e);
            Err(ErrorKind::Unexpected)
        }
        Ok(mut response) => {
            let body = response.body().await.unwrap().to_vec();

            match serde_json::from_slice::<Vec<SPIResourceResponseItem>>(&body) {
                Ok(data) => Ok(data),
                Err(e) => {
                    debug!(target: "apex", "Unexpected error parsing bulk authorize response: {}", e);
                    Err(ErrorKind::Unexpected)
                }
            }
        }
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
