use crate::app_config::AppConfig;
use crate::errors::ErrorKind;
use crate::importing::redis::create_redis_consumer;
use crate::rdf::iri_utils::stem_iri;
use crate::serving::bulk::{
    RefreshTokenRequest, RefreshTokenResponse, SPIBulkRequest, SPIResourceRequestItem,
    SPITenantFinderRequest, SPITenantFinderResponse,
};
use crate::serving::request_headers::HeaderCopy;
use crate::serving::ua::bulk_ua;
use actix_http::client::SendRequestError;
use actix_http::http::{header, StatusCode};
use actix_web::client::{Client, ClientRequest};
use actix_web::{web, HttpMessage};
use chrono::TimeZone;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use redis::Commands;
use ring::hmac;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::time::Duration;
use uuid::Uuid;

pub(crate) struct BulkCtx {
    pub(crate) req: actix_web::HttpRequest,
    pub(crate) config: web::Data<AppConfig>,
    current_tenant_path: Result<String, ErrorKind>,
    current_website: Result<String, ErrorKind>,
}

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
struct RedisSession {
    secret: String,
    #[serde(rename = "userToken")]
    user_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: String,
    #[serde(rename = "_expire")]
    expire: isize,
    #[serde(rename = "_maxAge")]
    max_age: isize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iat: i64,
    exp: i64,
}

impl BulkCtx {
    pub(crate) fn new(req: actix_web::HttpRequest, config: web::Data<AppConfig>) -> BulkCtx {
        BulkCtx {
            req,
            config,
            current_tenant_path: Err(ErrorKind::Unexpected),
            current_website: Err(ErrorKind::Unexpected),
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
        let core_api_host = env::var("ARGU_API_URL").unwrap();
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
            Err(_) => bail!(ErrorKind::Unexpected),
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

    async fn try_authenticate(&mut self) -> Result<String, ErrorKind> {
        match self.config.session_cookie_name.as_ref() {
            Some(name) => match self.req.cookie(&name) {
                Some(value) => match Uuid::parse_str(value.value()) {
                    Ok(session_id) => {
                        let session_id = session_id.to_hyphenated().to_string();
                        let signature_name = self
                            .config
                            .session_cookie_sig_name
                            .as_ref()
                            .expect("Session cookie configured without signature name");
                        let client_sig = self
                            .req
                            .cookie(&signature_name)
                            .expect("Session cookie without signature");
                        let secret = self.config.session_secret.as_ref().unwrap();
                        let key =
                            hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, &secret.as_bytes());

                        let sign = hmac::sign(&key, format!("{}={}", name, session_id).as_bytes());
                        // https://github.com/tj/node-cookie-signature/blob/master/index.js#L23
                        let generated_sig = base64::encode(&sign)
                            .replace('/', "_")
                            .replace('+', "-")
                            .replace('=', "");

                        if generated_sig == client_sig.value() {
                            let token = self.retrieve_and_validate_token(&session_id).await?;
                            Ok(token)
                        } else {
                            Err(ErrorKind::SecurityError(
                                "Invalid cookie signature value".into(),
                            ))
                        }
                    }
                    Err(_) => Err(ErrorKind::InvalidRequest),
                },
                None => Err(ErrorKind::ToDo),
            },
            None => Err(ErrorKind::ToDo),
        }
    }

    async fn retrieve_and_validate_token(&mut self, session_id: &str) -> Result<String, ErrorKind> {
        info!(target: "apex", "retrieve_and_validate_token 0");
        let enc_token = self
            .config
            .jwt_encryption_token
            .as_ref()
            .expect("No JWT encryption token");

        let mut redis = create_redis_consumer().expect("Failed to connect to redis");
        let value: String = redis.get(session_id).map_err(|_| ErrorKind::Unexpected)?;
        let token: RedisSession = serde_json::from_str(&value)
            .map_err(|_| ErrorKind::ParserError("Unexpected session value".into()))?;

        let jwt = decode::<Claims>(
            &token.user_token,
            &DecodingKey::from_secret(enc_token.as_ref()),
            &Validation::new(Algorithm::HS512),
        )
        .map_err(|e| {
            warn!(target: "apex", "Error decoding token: {} for token {}", e, token.user_token);
            ErrorKind::SecurityError("Invalid JWT signature".into())
        })?;

        if chrono::Utc::now() >= chrono::Utc.timestamp(jwt.claims.exp, 0) {
            return match self
                .refresh_token(&token.user_token, &token.refresh_token)
                .await
            {
                Ok(new_token) => {
                    let next_token = RedisSession {
                        refresh_token: new_token.refresh_token,
                        user_token: new_token.access_token.clone(),
                        ..token
                    };
                    let next_token =
                        serde_json::to_string(&next_token).expect("Error serializing new token");
                    let _: () = redis
                        .set(session_id, &next_token)
                        .expect("Error storing refreshed token");
                    Ok(new_token.access_token)
                }
                Err(_) => Err(ErrorKind::SecurityError("Expired token".into())),
            };
        }

        info!(target: "apex", "Verified JWT - exp: {}, curr: {}", chrono::Utc.timestamp(jwt.claims.exp, 0), chrono::Utc::now());
        Ok(token.user_token)
    }

    async fn refresh_token(
        &mut self,
        user_token: &str,
        refresh_token: &str,
    ) -> Result<RefreshTokenResponse, ErrorKind> {
        let client = Client::default();
        let core_api_host = env::var("ARGU_API_URL").unwrap();
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
