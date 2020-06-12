use crate::db::cache_control::CacheControl;
use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::doc_by_iri;
use crate::errors::ErrorKind;
use crate::hashtuple::{HashModel, LookupTable, Statement};
use crate::importing::importer::process_message;
use crate::importing::parsing::{parse_hndjson, DocumentSet};
use crate::rdf::iri_utils::stem_iri;
use crate::reporting::reporter::humanize;
use crate::serving::response_type::{ResponseType, NQUADS_MIME, NTRIPLES_MIME};
use crate::serving::responses::set_default_headers;
use crate::serving::serialization::{
    bulk_result_to_hextuples, bulk_result_to_nquads, bulk_result_to_ntriples,
};
use actix_http::error::BlockingError;
use actix_web::client::Client;
use actix_web::http::header;
use actix_web::{post, web, HttpResponse, Responder};
use futures::StreamExt;
use percent_encoding::percent_decode_str;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

struct BulkTiming {
    /// Parsing the request
    pub parse_time: Duration,
    /// Checking which resources are in the db
    pub lookup_time: Duration,
    /// Figuring out which resources need to be authorized and/or fetched
    pub sort_time: Duration,
    pub authorize_timing: Option<AuthorizeTiming>,
    /// Serializing the response
    pub serialize_time: Duration,
}

struct AuthorizeTiming {
    /// Calling the endpoints for authorization and/or fetching resources
    pub authorize_fetch_time: Duration,
    /// Parsing the fetched resources
    pub authorize_parse_time: Duration,
    /// Processing the data in the fetched resources (applying and storing)
    pub authorize_process_time: Duration,
    /// Consolidating the results back into the collected resources
    pub authorize_finish_time: Duration,
}

#[post("/link-lib/bulk")]
pub(crate) async fn bulk<'a>(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    payload: web::Payload,
) -> impl Responder {
    let parse_start = Instant::now();

    let pl = pool.clone().into_inner();

    let resources = match parse_request(payload).await {
        Ok(resources) => resources,
        Err(e) => return e,
    };

    debug!(target: "apex", "Requested {} resources", resources.len());
    let bulk_resources = resources.clone();

    let parse_end = Instant::now();
    let parse_time = parse_end.duration_since(parse_start);

    let (mut bulk_docs, mut lookup_table) = match lookup_resources(pl, bulk_resources).await {
        Ok(res) => (res.0, res.1),
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let lookup_end = Instant::now();
    let lookup_time = lookup_end.duration_since(parse_end);

    let (resources_in_cache, non_public_resources) = sort(resources, &mut bulk_docs);

    let sort_end = Instant::now();
    let sort_time = sort_end.duration_since(lookup_end);

    let authorize_timing = if non_public_resources.len() > 0 {
        let t = process_non_public_and_missing(
            &req,
            pool,
            lookup_table,
            &mut bulk_docs,
            &non_public_resources,
            &resources_in_cache,
        )
        .await;
        let (table, timing) = match t {
            Ok(t) => t,
            Err(res) => return res,
        };
        lookup_table = table;

        Some(timing)
    } else {
        debug!("All resources are public");
        None
    };

    let serialize_start = Instant::now();
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
    let serialize_time = Instant::now().duration_since(serialize_start);

    if cfg!(debug_assertions) {
        // TODO: async send to collection service or something
        let timing = BulkTiming {
            parse_time,
            lookup_time,
            sort_time,
            authorize_timing,
            serialize_time,
        };
        log_report(timing);
    }

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
    resources: &Vec<String>,
    resources_in_cache: &Vec<String>,
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

    let mut included: i32 = 0;
    let items = resources
        .into_iter()
        .map(stem_iri)
        .collect::<HashSet<String>>()
        .into_iter()
        .map(|iri| {
            let include = !resources_in_cache.contains(&iri);
            if include {
                included += 1;
            }
            SPIResourceRequestItem { include, iri }
        })
        .collect();
    let total = resources.len() as i32;
    debug!("test: auth: {}, include: {}", total, included);
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
                    if cfg!(debug_assertions) {
                        let output = String::from_utf8(body.to_vec()).unwrap();
                        debug!("Response body from server: {}", output);
                    }

                    Err(ErrorKind::Unexpected)
                }
            }
        }
    }
}

fn log_report(timing: BulkTiming) {
    let parse_msg = humanize("parse", timing.parse_time);
    let lookup_msg = humanize("lookup", timing.lookup_time);
    let sort_msg = humanize("sort", timing.sort_time);
    let auth_times = match timing.authorize_timing {
        Some(a) => (
            a.authorize_fetch_time,
            a.authorize_parse_time,
            a.authorize_process_time,
            a.authorize_finish_time,
        ),
        None => (
            Duration::new(0, 0),
            Duration::new(0, 0),
            Duration::new(0, 0),
            Duration::new(0, 0),
        ),
    };
    let auth_msg = format!(
        "{}{}{}{}",
        humanize("auth fetch", auth_times.0),
        humanize("auth parse", auth_times.1),
        humanize("auth process", auth_times.2),
        humanize("auth finish", auth_times.3)
    );
    let serialize_msg = humanize("serialize", timing.serialize_time);

    info!(target: "apex", "Bulk time: {}{}{}{}{}", parse_msg, lookup_msg, sort_msg, auth_msg, serialize_msg);
    let internal_time = timing.parse_time
        + timing.lookup_time
        + timing.sort_time
        + auth_times.1
        + auth_times.2
        + auth_times.3
        + timing.serialize_time;
    info!(target: "apex", "Bulk res: {}{}",
    humanize("internal", internal_time),
    humanize("external", auth_times.0),
    );
}

async fn lookup_resources(
    pl: Arc<DbPool>,
    bulk_resources: Vec<String>,
) -> Result<(Vec<Resource>, LookupTable), BlockingError<i32>> {
    web::block(move || -> Result<(Vec<Resource>, LookupTable), i32> {
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
    .await
}

async fn process_non_public_and_missing(
    req: &actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    mut lookup_table: LookupTable,
    bulk_docs: &mut Vec<Resource>,
    non_public_resources: &Vec<String>,
    resources_in_cache: &Vec<String>,
) -> Result<(LookupTable, AuthorizeTiming), actix_http::Response> {
    let authorize_start = Instant::now();

    debug!("Authorize {} documents", non_public_resources.len());
    let auth_result =
        match authorize_resources(&req, non_public_resources, &resources_in_cache).await {
            Ok(data) => data,
            Err(err) => {
                error!(target: "apex", "Unexpected error while authorizing: {}", err);
                return Err(HttpResponse::InternalServerError().finish());
            }
        };

    let authorize_fetch_end = Instant::now();
    let authorize_fetch_time = authorize_fetch_end.duration_since(authorize_start);

    // 7. RS saves resources with cache headers to db according to policy
    let uncached_and_included: Vec<&SPIResourceResponseItem> = auth_result
        .iter()
        .filter(|r| {
            debug!(target: "apex", "Auth result; iri: {}, status: {}, cache: {}, included: {}", r.iri, r.status, r.cache, r.body.is_some());
            r.status == 200 && !resources_in_cache.contains(&r.iri) && r.body.is_some()
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

                return Err(HttpResponse::InternalServerError().finish());
            }
        }
    }
    let authorize_parse_end = Instant::now();
    let authorize_parse_time = authorize_parse_end.duration_since(authorize_fetch_end);

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
                return Err(HttpResponse::InternalServerError().finish());
            }
        }

        lookup_table = ctx.lookup_table
    }
    let authorize_process_end = Instant::now();
    let authorize_process_time = authorize_process_end.duration_since(authorize_process_end);

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
    let authorize_finish_end = Instant::now();
    let authorize_finish_time = authorize_finish_end.duration_since(authorize_process_end);

    let timing = AuthorizeTiming {
        authorize_fetch_time,
        authorize_parse_time,
        authorize_process_time,
        authorize_finish_time,
    };

    Ok((lookup_table, timing))
}

async fn parse_request(payload: web::Payload) -> Result<Vec<String>, actix_http::Response> {
    let resource_set = resources_from_payload(payload).await;
    if resource_set.is_err() {
        return Err(HttpResponse::BadRequest().finish());
    };
    let resource_set = resource_set.unwrap();
    let resources: Vec<String> = resource_set
        .into_iter()
        .map(|r| String::from(percent_decode_str(&r).decode_utf8().unwrap()))
        .collect();

    Ok(resources)
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

fn sort<'a>(resources: Vec<String>, bulk_docs: &mut Vec<Resource>) -> (Vec<String>, Vec<String>) {
    let resources_in_cache: Vec<String> = bulk_docs
        .iter_mut()
        .filter(|r| r.status == 200)
        .map(|r| r.iri.clone())
        .collect();
    debug!("resources_in_cache {}", resources_in_cache.join(","));

    // 4. RS sends bulk authorize request to BE for all non-public resources (process The status code and cache headers per resource)
    let non_public_resources = resources
        .into_iter()
        .filter(|iri| {
            !bulk_docs.iter().any(|r| {
                r.cache_control == CacheControl::Public && r.iri.as_str() == stem_iri(iri).as_str()
            })
        })
        .collect();

    (resources_in_cache, non_public_resources)
}
