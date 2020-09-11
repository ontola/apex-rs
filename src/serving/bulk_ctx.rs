use crate::errors::ErrorKind;
use crate::rdf::iri_utils::stem_iri;
use crate::serving::bulk::{
    SPIBulkRequest, SPIResourceRequestItem, SPITenantFinderRequest, SPITenantFinderResponse,
};
use crate::serving::request_headers::HeaderCopy;
use crate::serving::ua::bulk_ua;
use actix_http::http::{header, StatusCode};
use actix_web::client::{Client, ClientRequest};
use std::collections::HashSet;
use std::env;
use std::time::Duration;

pub(crate) struct BulkCtx {
    pub(crate) req: actix_web::HttpRequest,
    current_tenant_path: Result<String, ErrorKind>,
    current_website: Result<String, ErrorKind>,
}

impl BulkCtx {
    pub(crate) fn new(req: actix_web::HttpRequest) -> BulkCtx {
        BulkCtx {
            req,
            current_tenant_path: Err(ErrorKind::Unexpected),
            current_website: Err(ErrorKind::Unexpected),
        }
    }

    pub(crate) fn website(&mut self) -> Result<String, ErrorKind> {
        match &self.current_website {
            Ok(iri) => Ok(iri.into()),
            Err(_) => match self.determine_website() {
                Ok(ref website) => {
                    self.current_website = Ok(website.into());
                    Ok(website.into())
                }
                Err(e) => Err(e),
            },
        }
    }

    pub(crate) async fn tenant_path(&mut self) -> Result<String, ErrorKind> {
        if let Ok(existing) = &self.current_tenant_path {
            return Ok(String::from(existing.clone()));
        }

        match self.determine_tenant_path().await {
            Err(e) => Err(ErrorKind::from(e)),
            Ok(ref tenant_path) => {
                let next = String::from(tenant_path);
                self.current_tenant_path = Ok(next.clone());
                Ok(next)
            }
        }
    }

    pub(crate) async fn setup_proxy_request(&mut self) -> Result<ClientRequest, ErrorKind> {
        let client = Client::default();
        let mut backend_req = client
            .post(format!(
                "{}{}/spi/bulk",
                self.config.data_server_url.clone(),
                self.tenant_path().await?
            ))
            .timeout(Duration::from_secs(self.config.data_server_timeout.clone()))
            .header("Website-IRI", self.website()?);

        let auth = self
            .req
            .headers()
            .get("authorization")
            .map(|s| s.to_str().unwrap());
        if let Some(auth) = auth {
            backend_req = backend_req.header(header::AUTHORIZATION, auth)
        }

        let backend_req = backend_req
            .copy_header_from("Accept-Language", &self.req, None)
            .copy_header_from("Origin", &self.req, None)
            .copy_header_from("Referer", &self.req, None)
            .copy_header_from("User-Agent", &self.req, Some(&bulk_ua()))
            .copy_header_from("X-Forwarded-Host", &self.req, None)
            .copy_header_from("X-Forwarded-Proto", &self.req, None)
            .copy_header_from("X-Forwarded-Ssl", &self.req, Some("on".into()))
            .copy_header_from("X-Real-Ip", &self.req, None)
            .copy_header_from("X-Requested-With", &self.req, None)
            .copy_header_from("X-Device-Id", &self.req, None)
            .copy_header_from("X-Request-Id", &self.req, None)
            .copy_header_from("X-Forwarded-For", &self.req, None)
            .copy_header_from("X-Client-Ip", &self.req, None)
            .copy_header_from("Client-Ip", &self.req, None)
            .copy_header_from("Host", &self.req, None)
            .copy_header_from("Forwarded", &self.req, None);

        Ok(backend_req)
    }

    pub(crate) fn compose_spi_bulk_payload(
        &mut self,
        resources: &Vec<String>,
        resources_in_cache: &Vec<String>,
    ) -> SPIBulkRequest {
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
        debug!("Documents; {} to authorize, {} to include", total, included);

        SPIBulkRequest { resources: items }
    }

    async fn determine_tenant_path(&mut self) -> Result<String, ErrorKind> {
        let client = Client::default();
        let core_api_host = env::var("ARGU_API_URL").unwrap();
        let tenant_req_body = SPITenantFinderRequest {
            iri: self.website()?.into(),
        };

        let req = client
            .get(format!("{}/_public/spi/find_tenant", core_api_host).as_str())
            .header(header::USER_AGENT, bulk_ua())
            .copy_header_from("X-Request-Id", &self.req, None);

        let mut tenant_res = req
            .send_json(&tenant_req_body)
            .await
            .expect("Error finding tenant");

        match tenant_res.status() {
            StatusCode::OK => {
                let tenant = tenant_res
                    .json::<SPITenantFinderResponse>()
                    .await
                    .expect("Error parsing tenant finder response");
                let url = format!("https://{}", tenant.iri_prefix);

                match url::Url::parse(url.as_str()) {
                    Ok(iri_prefix) => match String::from(iri_prefix.path()).as_str() {
                        "/" => Ok("".to_string()),
                        path => Ok(path.to_string()),
                    },
                    Err(_) => Err(ErrorKind::NoTenant),
                }
            }
            StatusCode::NOT_FOUND => Err(ErrorKind::NoTenant),
            _ => {
                debug!(target: "apex", "Unexpected status tenant finder: Got HTTP {}", tenant_res.status());
                Err(ErrorKind::Unexpected)
            }
        }
    }

    fn determine_website(&self) -> Result<String, ErrorKind> {
        let headers = self.req.headers();

        let authority = ["authority", "X-Forwarded-Host", "origin", "host", "accept"]
            .iter()
            .find(|header| headers.contains_key(String::from(**header)))
            .and_then(
                |header_key| match headers.get(*header_key).unwrap().to_str() {
                    Ok(authority) => {
                        let authority = authority.to_string();
                        if authority.contains(":") {
                            Some(authority)
                        } else {
                            headers
                                .get("X-Forwarded-Proto")
                                .and_then(|h| h.to_str().ok())
                                .or_else(|| headers.get("scheme").and_then(|h| h.to_str().ok()))
                                .and_then(|proto| Some(format!("{}://{}", proto, authority)))
                        }
                    }
                    Err(_) => None,
                },
            )
            .expect("Could not determine authority");

        if headers.contains_key("website-iri") {
            let website_iri: &str = match headers.get("website-iri") {
                None => bail!(ErrorKind::ParserError("Empty Website-Iri header".into())),
                Some(iri) => match iri.to_str() {
                    Ok(iri) => iri.into(),
                    Err(_) => bail!(ErrorKind::ParserError("Invalid Website-Iri value".into())),
                },
            };

            if !website_iri.starts_with(&authority) {
                let msg = format!(
                    "Website-Iri does not correspond with authority headers (website-iri {}, authority: {})",
                    website_iri,
                    authority,
                );
                bail!(ErrorKind::SecurityError(msg));
            }

            return Ok(website_iri.into());
        }

        Ok(authority)
    }
}
