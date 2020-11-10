use crate::app_config::AppConfig;
use crate::errors::ErrorKind;
use crate::rdf::iri_utils::stem_iri;
use crate::serving::bulk::{
    SPIBulkRequest, SPIResourceRequestItem, SPITenantFinderRequest, SPITenantFinderResponse,
};
use crate::serving::request_headers::HeaderCopy;
use crate::serving::sessions::{
    retrieve_session, session_id, session_info, RedisSession, RefreshTokenRequest,
    RefreshTokenResponse,
};
use crate::serving::ua::bulk_ua;
use actix_http::client::SendRequestError;
use actix_http::http::{header, StatusCode};
use actix_web::client::{Client, ClientRequest};
use actix_web::http::Method;
use actix_web::web;
use chrono::TimeZone;
use itertools::Itertools;
use redis::Commands;
use std::collections::HashSet;
use std::time::Duration;

pub(crate) struct BulkCtx {
    pub(crate) req: actix_web::HttpRequest,
    pub(crate) config: web::Data<AppConfig>,
    pub(crate) language: Option<String>,
    current_tenant_path: Result<String, ErrorKind>,
    current_website: Result<String, ErrorKind>,
}

impl BulkCtx {
    pub(crate) fn new(
        req: actix_web::HttpRequest,
        config: web::Data<AppConfig>,
        language: Option<String>,
    ) -> BulkCtx {
        BulkCtx {
            req,
            config,
            current_tenant_path: Err(ErrorKind::Unexpected("current_tenant_path not set".into())),
            current_website: Err(ErrorKind::Unexpected("current_website not set".into())),
            language,
        }
    }

    pub(crate) async fn authentication(&mut self) -> Result<String, ErrorKind> {
        if let Some(value) = self.req.headers().get("Authorization") {
            let token = value.to_str().map_err(|_| ErrorKind::InvalidRequest)?;
            if token.starts_with("Bearer ") {
                return Ok(String::from(token));
            }
        }

        match self.try_authenticate().await {
            Ok(token) => Ok(format!("Bearer {}", token)),
            Err(e) => {
                warn!(target: "apex", "Failed to authenticate: {}", e);
                match &self.config.service_guest_token {
                    Some(token) => Ok(format!("Bearer {}", token)),
                    None => Err(ErrorKind::Msg("No authentication".into())),
                }
            }
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
            Err(e) => Err(e),
            Ok(ref tenant_path) => {
                let next = String::from(tenant_path);
                self.current_tenant_path = Ok(next.clone());
                Ok(next)
            }
        }
    }

    pub(crate) async fn bulk_endpoint_url(&mut self) -> Result<String, ErrorKind> {
        let endpoint = format!(
            "{}{}/spi/bulk",
            self.config
                .data_server_url
                .clone()
                .expect("No data server url set"),
            self.tenant_path().await?
        );

        Ok(endpoint)
    }

    pub(crate) async fn setup_proxy_request(
        &mut self,
        method: Method,
        endpoint_url: String,
    ) -> Result<ClientRequest, ErrorKind> {
        let client = Client::default();
        let mut backend_req = client
            .request(method, endpoint_url)
            .timeout(Duration::from_secs(self.config.data_server_timeout.clone()))
            .header("Website-IRI", self.website()?);

        match self.authentication().await {
            Ok(auth) => backend_req = backend_req.header(header::AUTHORIZATION, auth),
            Err(e) => {
                warn!(target: "apex", "Error authenticating: {}", e);
            }
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
            // .copy_header_from("X-Forwarded-For", &self.req, None)
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
        let core_api_host = self
            .config
            .data_server_url
            .clone()
            .expect("No data server url set");
        let tenant_req_body = SPITenantFinderRequest {
            iri: self.website()?.into(),
        };

        let req = client
            .get(format!("{}/_public/spi/find_tenant", core_api_host).as_str())
            .header(header::USER_AGENT, bulk_ua())
            .copy_header_from("X-Request-Id", &self.req, None);

        let mut tenant_res = match req.send_json(&tenant_req_body).await {
            Ok(tenant_res) => tenant_res,
            Err(SendRequestError::Timeout) => bail!(ErrorKind::BackendUnavailable),
            Err(SendRequestError::Connect(_)) => bail!(ErrorKind::BackendUnavailable),
            Err(e) => bail!(ErrorKind::Unexpected(e.to_string())),
        };

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
            s => {
                debug!(target: "apex", "Unexpected status tenant finder: Got HTTP {}", tenant_res.status());
                Err(ErrorKind::Unexpected(format!(
                    "Unexpected status '{}'",
                    s.as_u16()
                )))
            }
        }
    }

    fn determine_website(&self) -> Result<String, ErrorKind> {
        let headers = self.req.headers();

        let header_key = ["authority", "X-Forwarded-Host", "origin", "host", "accept"]
            .iter()
            .find(|header| headers.contains_key(String::from(**header)))
            .expect("No header usable for authority present");
        let authority = match headers.get(*header_key).unwrap().to_str() {
            Ok(authority) => {
                let authority = authority.to_string();
                if authority.contains(":") {
                    Ok(authority)
                } else {
                    debug!(target: "apex", "Authority not complete: {}", authority);
                    let proto = if let Some(t) = headers.get("X-Forwarded-Proto") {
                        t.to_str().unwrap()
                    } else {
                        match headers.get("scheme") {
                            Some(scheme) => scheme.to_str().unwrap(),
                            None => {
                                let header_map = headers
                                    .iter()
                                    .map(|(k, v)| {
                                        format!(
                                            "{}: {}",
                                            k.to_string(),
                                            v.to_str().expect("Invalid header key")
                                        )
                                    })
                                    .join("\n");
                                debug!(target: "apex", "Headers: {}", header_map);
                                bail!("No forwarded proto nor scheme header")
                            }
                        }
                    };

                    Ok(format!("{}://{}", proto, authority))
                }
            }
            Err(e) => Err(e),
        };
        let authority = authority.unwrap();

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

    async fn try_authenticate(&mut self) -> Result<String, ErrorKind> {
        let session_id = session_id(&self.req)?;
        let token = self.retrieve_and_validate_token(&session_id).await?;
        Ok(token)
    }

    async fn retrieve_and_validate_token(&mut self, session_id: &str) -> Result<String, ErrorKind> {
        let mut redis = self
            .config
            .create_redis_consumer()
            .expect("Failed to connect to redis");
        let token = retrieve_session(&self.config, session_id).await?;

        match session_info(session_id).await {
            Ok(claims) => {
                debug!(target: "apex", "Verified JWT - exp: {}, curr: {}", chrono::Utc.timestamp(claims.exp, 0), chrono::Utc::now());
                Ok(token.user_token)
            }
            Err(ErrorKind::ExpiredSession) => {
                match self
                    .refresh_token(&token.user_token, &token.refresh_token)
                    .await
                {
                    Ok(new_token) => {
                        let next_token = RedisSession {
                            refresh_token: new_token.refresh_token,
                            user_token: new_token.access_token.clone(),
                            ..token
                        };
                        let next_token = serde_json::to_string(&next_token)
                            .expect("Error serializing new token");
                        let _: () = redis
                            .set(session_id, &next_token)
                            .expect("Error storing refreshed token");
                        Ok(new_token.access_token)
                    }
                    Err(_) => Err(ErrorKind::SecurityError("Expired token".into())),
                }
            }
            Err(e) => Err(e),
        }
    }

    async fn refresh_token(
        &mut self,
        user_token: &str,
        refresh_token: &str,
    ) -> Result<RefreshTokenResponse, ErrorKind> {
        let client = Client::default();
        let core_api_host = self
            .config
            .data_server_url
            .clone()
            .expect("No data server url set");
        let refresh_token_req = RefreshTokenRequest {
            client_id: self
                .config
                .client_id
                .as_ref()
                .expect("No client_id configured")
                .into(),
            client_secret: self
                .config
                .client_secret
                .as_ref()
                .expect("No client_secret configured")
                .into(),
            grant_type: "refresh_token".into(),
            refresh_token: refresh_token.into(),
        };

        let req = client
            .get(format!("{}/oauth/token", core_api_host).as_str())
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, user_token)
            .header(header::USER_AGENT, bulk_ua())
            .copy_header_from("X-Forwarded-Host", &self.req, None)
            .copy_header_from("X-Forwarded-Proto", &self.req, None)
            .copy_header_from("X-Forwarded-Ssl", &self.req, Some("on".into()))
            .copy_header_from("X-Request-Id", &self.req, None);

        let mut tenant_res = req
            .send_json(&refresh_token_req)
            .await
            .expect("Error refreshing token");

        match tenant_res.status() {
            StatusCode::OK => {
                let token = tenant_res
                    .json::<RefreshTokenResponse>()
                    .await
                    .expect("Error parsing refresh token response");

                Ok(token)
            }
            status => {
                error!(target: "apex", "Non 200 status: {}", status);
                Err(ErrorKind::ToDo)
            }
        }
    }
}
