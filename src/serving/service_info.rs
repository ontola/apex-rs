use crate::serving::response_type::ResponseType::JSONLD;
use crate::serving::responses::set_default_headers;
use actix_web::{get, HttpResponse, Responder};
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Envelope {
    /// The name of the service
    #[serde(borrow)]
    name: &'static str,
    /// The operators the service supports
    #[serde(borrow)]
    operators: Vec<&'static str>,
    /// The operator arguments the service supports
    #[serde(borrow)]
    arguments: Vec<&'static str>,
    /// The endpoints of the service
    endpoints: EndpointMap,
}

#[derive(Deserialize, Serialize)]
struct EndpointMap {
    /// Bulk endpoint
    pub bulk: Option<EndpointInformation>,
    /// Triple pattern fragments endpoint
    pub tpf: Option<EndpointInformation>,
    /// Hex pattern fragments endpoint
    pub hpf: Option<EndpointInformation>,
}

#[derive(Deserialize, Serialize)]
struct EndpointInformation {
    path: String,
    method: String,
    stability: EndpointStability,
    content_types: ContentTypeMap,
    info: Option<String>,
}

#[derive(Deserialize, Serialize)]
enum EndpointStability {
    /// Endpoint is stable, has (paid) organizational support
    Supported,
    /// Endpoint is stable
    Stable,
    /// Endpoint stability cannot be relied upon yet
    Unstable,
    /// Endpoint is (still) experimental
    Experimental,
}

#[derive(Copy, Clone, Deserialize, Serialize)]
struct ContentTypeMap {
    #[serde(rename = "text/turtle")]
    turtle: bool,
    #[serde(rename = "application/hex+x-ndjson")]
    hex: bool,
    #[serde(rename = "application/n-quads")]
    nquads: bool,
    #[serde(rename = "application/n-triples")]
    ntriples: bool,
    #[serde(rename = "application/ld+json")]
    jsonld: bool,
    #[serde(rename = "application/rdf+json")]
    rdfjson: bool,
    #[serde(rename = "application/rdf+xml")]
    rdfxml: bool,
    #[serde(rename = "text/n3")]
    n3: bool,
}

/// Linked Delta informational endpoint
#[get("/.well-known/ld")]
pub(crate) async fn service_info<'a>() -> impl Responder {
    let ct_map = ContentTypeMap {
        turtle: true,
        hex: true,
        nquads: true,
        ntriples: true,
        jsonld: false,
        rdfjson: false,
        rdfxml: false,
        n3: false,
    };

    let name = "Apex/1";
    let operators = vec![
        "http://purl.org/linked-delta/add",
        "http://purl.org/linked-delta/replace",
    ];
    let arguments = vec!["graph"];
    let endpoints = EndpointMap {
        bulk: Some(EndpointInformation {
            path: "/link-lib/bulk".into(),
            method: "POST".into(),
            content_types: ct_map,
            stability: EndpointStability::Supported,
            info: None,
        }),
        tpf: Some(EndpointInformation {
            path: "/tpf".into(),
            method: "POST".into(),
            content_types: ct_map,
            stability: EndpointStability::Experimental,
            info: None,
        }),
        hpf: Some(EndpointInformation {
            path: "/hpf".into(),
            method: "POST".into(),
            content_types: ct_map,
            stability: EndpointStability::Experimental,
            info: None,
        }),
    };

    let body = Envelope {
        name,
        operators,
        arguments,
        endpoints,
    };

    set_default_headers(&mut HttpResponse::Ok(), &JSONLD).json(body)
}
