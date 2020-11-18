use crate::db::db_context::{DbContext, DbPool};
use crate::db::hpf::{HPFQuery, HPFQueryRequest, TPFQueryRequest};
use crate::errors::ErrorKind;
use crate::hashtuple::{HashModel, LookupTable};
use crate::serving::response_type::{ResponseType, NQUADS_MIME, NTRIPLES_MIME};
use crate::serving::responses::set_default_headers;
use crate::serving::serialization::{
    bulk_result_to_hextuples, bulk_result_to_nquads, bulk_result_to_ntriples,
};
use actix_web::error::BlockingError;
use actix_web::http::{header, HeaderMap, HeaderValue};
use actix_web::{get, web, HttpResponse, Responder};
use humantime::format_duration;
use std::env;
use std::time::Instant;

#[get("/hpf")]
pub(crate) async fn hpf(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    payload: web::Query<HPFQueryRequest>,
) -> impl Responder {
    let origin = origin_or_default(req.headers());
    let pl = pool.into_inner();

    let res = web::block(move || -> Result<(HashModel, LookupTable), ErrorKind> {
        let mut ctx = DbContext::new(&pl);
        let query = HPFQuery::parse(&mut ctx, &payload)?;

        fetch(ctx, &origin, query)
    })
    .await;

    respond(req, res)
}

#[get("/tpf")]
pub(crate) async fn tpf(
    req: actix_web::HttpRequest,
    pool: web::Data<DbPool>,
    payload: web::Query<TPFQueryRequest>,
) -> impl Responder {
    let origin = origin_or_default(req.headers());
    let pl = pool.into_inner();

    let res = web::block(move || -> Result<(HashModel, LookupTable), ErrorKind> {
        let mut ctx = DbContext::new(&pl);
        let query = HPFQuery::parse_tpf(&mut ctx, &payload)?;

        fetch(ctx, &origin, query)
    })
    .await;

    respond(req, res)
}

fn fetch(
    mut ctx: DbContext,
    origin: &str,
    query: HPFQuery,
) -> Result<(HashModel, LookupTable), ErrorKind> {
    let fetch_start = Instant::now();
    let models = query.execute(&mut ctx)?;
    let fetch_time = Instant::now().duration_since(fetch_start);
    debug!(target: "apex", "Fetching cost: {}", format_duration(fetch_time));
    let header = query.header(&mut ctx, &origin)?;

    let doc = [header.as_slice(), models.as_slice()].concat();

    Ok((doc, ctx.lookup_table))
}

fn respond(
    req: actix_web::HttpRequest,
    res: Result<(HashModel, LookupTable), BlockingError<ErrorKind>>,
) -> impl Responder {
    if res.is_err() {
        let err = res.err().unwrap();
        println!("Caught error: {:?}", err);
        return HttpResponse::InternalServerError().finish();
    }

    let (model, table) = res.unwrap();
    let bulk_arg = (vec![Some(model)], table);

    let convert_start = Instant::now();
    let (body, response_type) = if let Some(accept) = req.headers().get(header::ACCEPT) {
        let accept = accept.to_str().unwrap();
        if accept == NQUADS_MIME {
            (bulk_result_to_nquads(bulk_arg), ResponseType::NQUADS)
        } else if accept == NTRIPLES_MIME {
            (bulk_result_to_ntriples(bulk_arg), ResponseType::NTRIPLES)
        } else {
            (bulk_result_to_hextuples(bulk_arg), ResponseType::HEXTUPLE)
        }
    } else {
        (bulk_result_to_hextuples(bulk_arg), ResponseType::HEXTUPLE)
    };
    let convert_time = Instant::now().duration_since(convert_start);
    debug!(target: "apex", "Converting cost: {}", format_duration(convert_time));

    set_default_headers(&mut HttpResponse::Ok(), &response_type).body(body)
}

fn origin_or_default(headers: &HeaderMap) -> String {
    let default_host = env::var("HOSTNAME").expect("No default hostname given");
    let default_origin = format!("https://{}", default_host);
    let default_as_header = HeaderValue::from_str(&default_origin).unwrap();
    let origin = headers
        .get("Origin")
        .unwrap_or(&default_as_header)
        .to_str()
        .unwrap_or(&default_origin);

    String::from(origin)
}
