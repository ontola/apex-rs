use crate::app_config::AppConfig;
use crate::db::cache_control::CacheControl;
use crate::db::db_context::{DbContext, DbPool};
use crate::db::document::{doc_by_iri, update_cache_control};
use crate::errors::ErrorKind;
use crate::hashtuple::{HashModel, LookupTable, Statement};
use crate::importing::importer::process_message;
use crate::importing::parsing::{parse_hndjson, DocumentSet};
use crate::models::Document;
use crate::rdf::iri_utils::stem_iri;
use crate::serving::bulk_ctx::BulkCtx;
use crate::serving::reporter::Reporter;
use crate::serving::response_type::{ResponseType, NQUADS_MIME, NTRIPLES_MIME};
use crate::serving::responses::set_default_headers;
use crate::serving::route::route;
use crate::serving::serialization::{
    bulk_result_to_hextuples, bulk_result_to_nquads, bulk_result_to_ntriples,
};
use crate::serving::sessions::{session_id, session_info};
use crate::serving::timings::{AuthorizeTiming, BulkTiming};
use actix_http::error::BlockingError;
use actix_web::client::SendRequestError;
use actix_web::http::{header, Method};
use actix_web::{post, web, HttpResponse, Responder};
use futures::StreamExt;
use log::Level;
use percent_encoding::percent_decode_str;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

#[derive(Deserialize, Serialize)]
pub(crate) struct FormData {
    resource: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct SPIResourceRequestItem {
    pub iri: String,
    pub include: bool,
}

#[derive(Serialize)]
pub(crate) struct SPIBulkRequest {
    pub resources: Vec<SPIResourceRequestItem>,
}

#[derive(Serialize)]
pub(crate) struct SPITenantFinderRequest {
    pub iri: String,
}

#[derive(Deserialize)]
pub(crate) struct SPITenantFinderResponse {
    pub iri_prefix: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct SPIResourceResponseItem {
    iri: String,
    status: u16,
    cache: CacheControl,
    language: Option<String>,
    body: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct SPIError {
    error: String,
    error_description: String,
}

pub(crate) struct Resource {
    iri: String,
    status: u16,
    cache_control: CacheControl,
    data: HashModel,
}

#[post("/link-lib/bulk")]
pub(crate) async fn bulk<'a>(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    config: web::Data<AppConfig>,
    reporter: web::Data<Reporter>,
    payload: web::Payload,
) -> impl Responder {
    let parse_start = Instant::now();
    reporter.register_bulk_request();

    let pl = pool.clone().into_inner();

    let lang = match session_id(&req) {
        Ok(sid) => match session_info(&sid).await {
            Ok(info) => Some(info.user.language),
            Err(e) => match e {
                ErrorKind::ExpiredSession => {
                    // TODO: REFRESH
                    debug!(target: "apex", "EXPIRED SESSION");
                    None
                }
                _ => Some(String::from("en")), // TODO: Take from manifest
            },
        },
        Err(_) => Some(String::from("en")),
    };
    let mut req = BulkCtx::new(req, config, lang);

    let resources = match parse_request(payload).await {
        Ok(resources) => resources,
        Err(e) => return e,
    };

    reporter.register_bulk_resource_count(resources.len());
    debug!(target: "apex", "Requested {} resources", resources.len());
    if log_enabled!(Level::Trace) {
        trace!(target: "apex", "Resources requested: {}", resources.clone().join(", "));
    }
    let bulk_resources = resources.clone();

    let parse_end = Instant::now();
    let parse_time = parse_end.duration_since(parse_start);

    let (mut bulk_docs, mut lookup_table) = match lookup_resources(&req, pl, bulk_resources).await {
        Ok(res) => (res.0, res.1),
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let lookup_end = Instant::now();
    let lookup_time = lookup_end.duration_since(parse_end);

    let (resources_in_store, private_or_missing) = sort(resources, &mut bulk_docs);

    if log_enabled!(Level::Trace) {
        trace!(target: "apex", "Resources in store: {}, missing: {}", &resources_in_store.join(", "), &private_or_missing.join(", "));
    }

    let sort_end = Instant::now();
    let sort_time = sort_end.duration_since(lookup_end);

    let authorize_timing = if private_or_missing.len() > 0 {
        let t = process_private_and_missing(
            &mut req,
            pool,
            lookup_table,
            &mut bulk_docs,
            &private_or_missing,
            &resources_in_store,
        )
        .await;
        let (table, timing) = match t {
            Ok(t) => t,
            Err(res) => return res,
        };
        lookup_table = table;

        Some(timing)
    } else {
        debug!(target: "apex", "All resources are present and public");
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

    let (body, response_type) = if let Some(accept) = req.req.headers().get(header::ACCEPT) {
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

    let timing = BulkTiming::from_durations(
        parse_time,
        lookup_time,
        sort_time,
        authorize_timing,
        serialize_time,
    );
    timing.report();
    reporter.add_bulk_timing(timing);

    set_default_headers(&mut HttpResponse::Ok(), &response_type).body(body)
}

fn status_code_statement(lookup_table: &mut LookupTable, iri: &str, status: u16) -> Statement {
    Statement {
        subject: lookup_table.ensure_value(iri),
        predicate: lookup_table.ensure_value("http://www.w3.org/2011/http#statusCode"),
        value: lookup_table.ensure_value(status.to_string().as_str()),
        datatype: lookup_table.ensure_value("http://www.w3.org/2001/XMLSchema#integer"),
        language: lookup_table.ensure_value(""),
        graph: lookup_table.ensure_value("http://purl.org/link-lib/meta"),
    }
}

async fn authorize_partitioned(
    req: &mut BulkCtx,
    resources: &Vec<String>,
    resources_in_store: &Vec<String>,
) -> Result<Vec<SPIResourceResponseItem>, ErrorKind> {
    let mut bulk_resources = Vec::new();
    let mut http_resources = Vec::new();

    for resource in resources {
        match route(&req.config.cluster_config, resource)? {
            Some(_) => http_resources.push(resource.clone()),
            None => bulk_resources.push(resource.clone()),
        }
    }

    let mut bulk_result = authorize_via_bulk(req, &bulk_resources, resources_in_store).await?;
    let mut http_result = authorize_via_http(req, &http_resources).await?;

    bulk_result.append(&mut http_result);

    Ok(bulk_result)
}

async fn authorize_via_http(
    req: &mut BulkCtx,
    resources: &Vec<String>,
) -> Result<Vec<SPIResourceResponseItem>, ErrorKind> {
    if resources.len() == 0 {
        return Ok(Vec::with_capacity(0));
    }
    let mut response_items = Vec::with_capacity(resources.len());

    let config = req.config.cluster_config.clone();
    for resource in resources {
        let url = route(&config, resource)?.unwrap();
        let backend_req = req
            .setup_proxy_request(Method::GET, url)
            .await?
            .header(header::ACCEPT, "application/hex+x-ndjson");
        let response = backend_req.send().await;

        match response {
            Err(SendRequestError::Timeout) => {
                debug!(target: "apex", "Timeout waiting for sending bulk authorize");
                return Err(ErrorKind::Timeout);
            }
            Err(e) => {
                debug!(target: "apex", "Unexpected error sending bulk authorize: {}", e);
                return Err(ErrorKind::Unexpected(e.to_string()));
            }
            Ok(mut response) => {
                let body = match response.body().limit(100_000_000).await {
                    Ok(body) => String::from_utf8(body.to_vec()).map_err(|_| {
                        ErrorKind::ParserError("Error decoding http response body as utf-8".into())
                    })?,
                    Err(e) => {
                        warn!(target: "apex", "Error while decoding backend auth response: {}", e);

                        return Err(ErrorKind::Unexpected(e.to_string()));
                    }
                };

                let item = SPIResourceResponseItem {
                    body: Some(body),
                    cache: CacheControl::Private,
                    iri: resource.into(),
                    language: None,
                    status: response.status().as_u16(),
                };

                response_items.push(item);
            }
        }
    }

    Ok(response_items)
}

async fn authorize_via_bulk(
    req: &mut BulkCtx,
    resources: &Vec<String>,
    resources_in_store: &Vec<String>,
) -> Result<Vec<SPIResourceResponseItem>, ErrorKind> {
    if resources.len() == 0 {
        return Ok(Vec::with_capacity(0));
    }

    let website = req.website()?;
    debug!(target: "apex", "Using website: {}", website);

    // Find tenant
    let tenant_path = req.tenant_path().await?;
    trace!(target: "apex", "Tenant: {}", tenant_path);

    // Create request builder and send request
    let bulk_url = req.bulk_endpoint_url().await?;
    let backend_req = req.setup_proxy_request(Method::POST, bulk_url).await?;
    let request_body = req.compose_spi_bulk_payload(&resources, &resources_in_store);
    let response = backend_req.send_json(&request_body).await;

    match response {
        Err(SendRequestError::Timeout) => {
            debug!(target: "apex", "Timeout waiting for sending bulk authorize");
            Err(ErrorKind::Timeout)
        }
        Err(e) => {
            debug!(target: "apex", "Unexpected error sending bulk authorize: {}", e);
            Err(ErrorKind::Unexpected(e.to_string()))
        }
        Ok(mut response) => {
            let body = match response.body().limit(100_000_000).await {
                Ok(body) => body.to_vec(),
                Err(e) => {
                    warn!(target: "apex", "Error while decoding backend auth response: {}", e);

                    return Err(ErrorKind::Unexpected(e.to_string()));
                }
            };

            match response.status().as_u16() {
                200 => match serde_json::from_slice::<Vec<SPIResourceResponseItem>>(&body) {
                    Ok(data) => Ok(data),
                    Err(e) => {
                        debug!(target: "apex", "Unexpected error parsing bulk authorize response: {} with body: {}", e, String::from_utf8(body.clone()).unwrap());
                        if cfg!(debug_assertions) {
                            let output = String::from_utf8(body.to_vec()).unwrap();
                            debug!(target: "apex", "Response body from server: {}", output);
                        }

                        Err(ErrorKind::Unexpected(e.to_string()))
                    }
                },
                _ => match serde_json::from_slice::<SPIError>(&body) {
                    Ok(e) => {
                        warn!(target: "apex", "Bulk authorize error: {} with description: {}", e.error, e.error_description);
                        Err(ErrorKind::Unexpected(e.error))
                    }
                    Err(e) => {
                        error!(target: "apex", "Unexpected error parsing bulk authorize error: {} with body: {}", e, String::from_utf8(body.clone()).unwrap());
                        Err(ErrorKind::Unexpected(e.to_string()))
                    }
                },
            }
        }
    }
}

async fn lookup_resources(
    req: &BulkCtx,
    pl: Arc<DbPool>,
    bulk_resources: Vec<String>,
) -> Result<(Vec<Resource>, LookupTable), BlockingError<i32>> {
    let disable_persistence = req.config.disable_persistence.clone();
    let lang = req.language.clone();

    web::block(move || -> Result<(Vec<Resource>, LookupTable), i32> {
        let mut ctx = DbContext::new_with_lang(&pl, lang);
        let resources = bulk_resources.into_iter().map(stem_iri);

        let models: Vec<Resource> = if disable_persistence {
            resources
                .map(|iri| Resource {
                    iri,
                    status: 404,
                    cache_control: CacheControl::Private,
                    data: HashModel::new(),
                })
                .collect()
        } else {
            resources
                .map(|iri| match doc_by_iri(&mut ctx, &iri) {
                    Ok((doc, data)) => {
                        trace!(target: "apex", "Load success: {}", iri);
                        Resource {
                            iri,
                            status: if data.is_empty() { 204 } else { 200 },
                            cache_control: doc.cache_control.into(),
                            data,
                        }
                    }
                    Err(ErrorKind::EmptyDocument) => {
                        trace!(target: "apex", "Load failed emtpy: {}", iri);
                        Resource {
                            iri,
                            status: 404,
                            cache_control: CacheControl::Private,
                            data: HashModel::new(),
                        }
                    }
                    Err(e) => {
                        trace!(target: "apex", "Load failed: {}, {}", iri, e);
                        Resource {
                            iri,
                            status: 500,
                            cache_control: CacheControl::Private,
                            data: HashModel::new(),
                        }
                    }
                })
                .collect()
        };

        Ok((models, ctx.lookup_table))
    })
    .await
}

async fn process_private_and_missing(
    mut req: &mut BulkCtx,
    pool: web::Data<DbPool>,
    mut lookup_table: LookupTable,
    bulk_docs: &mut Vec<Resource>,
    non_public_resources: &Vec<String>,
    resources_in_store: &Vec<String>,
) -> Result<(LookupTable, AuthorizeTiming), actix_http::Response> {
    let authorize_start = Instant::now();

    trace!(target: "apex", "Authorize / fetch {} documents", non_public_resources.len());
    let auth_result =
        match authorize_partitioned(&mut req, non_public_resources, &resources_in_store).await {
            Ok(data) => data,
            Err(ErrorKind::NoTenant) => {
                debug!(target: "apex", "Couldn't determine tenant");
                let mut body = bulk_docs
                    .iter()
                    .map(|r| Resource {
                        iri: r.iri.clone(),
                        status: 404,
                        cache_control: CacheControl::Private,
                        data: vec![],
                    })
                    .collect::<Vec<Resource>>();
                bulk_docs.clear();
                bulk_docs.append(&mut body);

                return Ok((lookup_table, AuthorizeTiming::default()));
            }
            Err(ErrorKind::ParserError(msg)) => {
                debug!(target: "apex", "Error while authorizing: {}", msg);
                return Err(HttpResponse::BadRequest().finish());
            }
            Err(ErrorKind::BackendUnavailable) => return Err(HttpResponse::BadGateway().finish()),
            Err(ErrorKind::Timeout) => return Err(HttpResponse::GatewayTimeout().finish()),
            Err(err) => {
                error!(target: "apex", "Unexpected error while authorizing: {}", err);
                return Err(HttpResponse::InternalServerError().finish());
            }
        };

    let authorize_fetch_end = Instant::now();
    let authorize_fetch_time = authorize_fetch_end.duration_since(authorize_start);

    // 7. RS saves resources with cache headers to db according to policy
    let unstored_and_included: Vec<&SPIResourceResponseItem> = auth_result
        .iter()
        .map(|r| {
            match r.status {
                200 | 204 => (),
                300..=399 => update_or_insert_doc(bulk_docs, &r.iri, r.status, r.cache, vec![]),
                _ => {
                    match &r.body {
                        Some(body) => {
                            let body = String::from(body);
                            match parse_hndjson(&mut lookup_table, body.as_ref()) {
                                Ok(data) => update_or_insert_doc(bulk_docs, &r.iri, r.status, r.cache, docset_to_model(data)),
                                Err(e) => warn!(target: "apex", "Error while processing bulk request {}", e),
                            }
                        }
                        None => {
                            update_or_insert_doc(bulk_docs, &r.iri, r.status, r.cache, vec![])
                        }
                    }
                }
            }

            r
        })
        .filter(|r| {
            trace!(target: "apex", "Auth result; iri: {}, status: {}, cache: {}, included: {}", r.iri, r.status, r.cache, r.body.is_some());
            r.status == 200 && !resources_in_store.contains(&r.iri) && r.body.is_some()
        })
        .collect();

    let mut unstored_and_included_documents = vec![];

    for r in &unstored_and_included {
        let body = r.body.as_ref().unwrap();
        match parse_hndjson(&mut lookup_table, body.as_ref()) {
            Ok(data) => {
                trace!(target: "apex", "parsed: {}", r.iri);
                unstored_and_included_documents.push(Document {
                    iri: r.iri.clone(),
                    status: r.status,
                    cache_control: r.cache,
                    language: r.language.clone(),
                    data: docset_to_model(data),
                });
            }
            Err(e) => {
                debug!(target: "apex", "Error while processing bulk request {}", e);

                return Err(HttpResponse::InternalServerError().finish());
            }
        }
    }
    let authorize_parse_end = Instant::now();
    let authorize_parse_time = authorize_parse_end.duration_since(authorize_fetch_end);

    let unstored_and_storable: Vec<Document> = unstored_and_included
        .iter()
        .enumerate()
        .filter(|(_, r)| r.cache != CacheControl::Private)
        .map(|(n, _)| unstored_and_included_documents.get(n).unwrap().clone())
        .collect();

    if !req.config.disable_persistence && !unstored_and_storable.is_empty() {
        trace!(target: "apex", "Storing {} new resources", unstored_and_storable.len());
        let pl = pool.into_inner();
        let mut ctx = DbContext::new_with_lang(&pl, req.language.clone());
        ctx.lookup_table = lookup_table;

        for doc in &unstored_and_storable {
            trace!(target: "apex", "Storing {} with cache control {}", doc.iri, doc.cache_control);
            let docset = document_to_docset(doc);
            if let Err(e) = process_message(&mut ctx, docset).await {
                error!(target: "apex", "Error writing resource to database: {}", e);
                return Err(HttpResponse::InternalServerError().finish());
            }
        }

        update_cache_control(&ctx.get_conn(), &unstored_and_storable);

        lookup_table = ctx.lookup_table
    }
    let authorize_process_end = Instant::now();
    let authorize_process_time = authorize_process_end.duration_since(authorize_process_end);

    for document in unstored_and_included_documents.into_iter() {
        trace!(target: "apex", "Including unstored document: {} as status {}", document.iri, document.status);
        update_or_insert_doc(
            bulk_docs,
            document.iri.as_str(),
            document.status,
            document.cache_control,
            document.data,
        );
    }
    let authorize_finish_end = Instant::now();
    let authorize_finish_time = authorize_finish_end.duration_since(authorize_process_end);

    let timing = AuthorizeTiming::from_durations(
        authorize_fetch_time,
        authorize_parse_time,
        authorize_process_time,
        authorize_finish_time,
    );

    Ok((lookup_table, timing))
}

fn document_to_docset(doc: &Document) -> DocumentSet {
    let mut docset = DocumentSet::new();
    docset.insert(doc.iri.clone(), doc.data.clone());
    docset
}

fn docset_to_model(docset: DocumentSet) -> HashModel {
    let mut data = Vec::new();
    for mut doc in docset {
        data.append(&mut doc.1);
    }
    data
}

fn update_or_insert_doc(
    bulk_docs: &mut Vec<Resource>,
    iri: &str,
    status: u16,
    cache: CacheControl,
    data: HashModel,
) {
    if let Some(d) = bulk_docs.iter_mut().find(|d| d.iri == iri) {
        d.status = status;
        d.cache_control = cache;
        d.data = data;
    } else {
        bulk_docs.push(Resource {
            iri: iri.into(),
            status,
            cache_control: cache,
            data,
        })
    }
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
    let resources_in_store: Vec<String> = bulk_docs
        .iter_mut()
        .filter(|r| r.status == 200 && !r.data.is_empty())
        .map(|r| r.iri.clone())
        .collect();
    trace!(
        "Non-empty resources already in cache: {}",
        resources_in_store.join(", ")
    );

    // 4. RS sends bulk authorize request to BE for all non-public resources (process The status code and cache headers per resource)
    let private_or_missing = resources
        .into_iter()
        .filter(|iri| {
            !bulk_docs.iter().any(|r| {
                r.cache_control == CacheControl::Public
                    && r.status == 200
                    && r.iri.as_str() == stem_iri(iri).as_str()
            })
        })
        .collect();

    (resources_in_store, private_or_missing)
}
