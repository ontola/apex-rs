use crate::app_config::AppConfig;
use crate::errors::ErrorKind;
use actix_web::{HttpMessage, HttpRequest};
use chrono::TimeZone;
use jsonwebtoken::{decode, Algorithm, DecodingKey, TokenData, Validation};
use redis::Commands;
use ring::hmac;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A session pair consisting of (session_id, validation_hash)
type SessionPair = (String, String);

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RedisSession {
    pub secret: Option<String>,
    #[serde(rename = "userToken")]
    pub user_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "_expire")]
    pub expire: isize,
    #[serde(rename = "_maxAge")]
    pub max_age: isize,
}

#[derive(Serialize)]
pub(crate) struct RefreshTokenRequest {
    pub client_id: String,
    pub client_secret: String,
    pub grant_type: String,
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub(crate) struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    #[serde(rename = "type")]
    pub user_type: String,
    #[serde(rename = "@id")]
    pub iri: String,
    pub id: String,
    pub email: Option<String>,
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub application_id: i64,
    pub exp: i64,
    pub iat: i64,
    pub scopes: Vec<String>,
    pub user: UserData,
}

pub fn session_id(req: &actix_web::HttpRequest) -> Result<String, ErrorKind> {
    let config = AppConfig::default();
    let (session_id, session_signature) = session_pair_from_req(&config, &req)?;
    verify_session_signature(&config, &session_id, &session_signature)?;

    Ok(session_id.into())
}

/// Retrieves, decodes, and validates the session from redis
pub async fn session_info(session_id: &str) -> Result<Claims, ErrorKind> {
    let config = AppConfig::default();
    let session = retrieve_session(&config, session_id).await?;
    let jwt = decode_session(&config, &session)?;

    if is_expired(jwt.claims.exp) {
        return Err(ErrorKind::ExpiredSession);
    }

    Ok(jwt.claims)
}

pub(crate) async fn retrieve_session(
    config: &AppConfig,
    session_id: &str,
) -> Result<RedisSession, ErrorKind> {
    let mut redis = config.create_redis_consumer().unwrap();
    let value: String = redis
        .get(session_id)
        .map_err(|_| ErrorKind::Msg(format!("Session {} not found in redis", session_id)))?;

    let session: RedisSession = serde_json::from_str(&value)
        .map_err(|_| ErrorKind::ParserError("Unexpected session value".into()))?;

    Ok(session)
}

fn is_expired(time: i64) -> bool {
    chrono::Utc::now() >= chrono::Utc.timestamp(time, 0)
}

fn decode_session(
    config: &AppConfig,
    session: &RedisSession,
) -> Result<TokenData<Claims>, ErrorKind> {
    let enc_token = config
        .jwt_encryption_token
        .as_ref()
        .expect("No JWT encryption token");

    decode::<Claims>(
        &session.user_token,
        &DecodingKey::from_secret(enc_token.as_ref()),
        &Validation::new(Algorithm::HS512),
    )
    .map_err(|e: jsonwebtoken::errors::Error| match e.into_kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => ErrorKind::ExpiredSession,
        _ => ErrorKind::SecurityError("Invalid JWT signature".into()),
    })
    .map_err(|e| {
        warn!(target: "apex", "Error decoding token: {} for token {}", e, session.user_token);
        ErrorKind::SecurityError("Other error during JWT signature decoding".into())
    })
}

fn session_pair_from_req(
    config: &AppConfig,
    req: &actix_web::HttpRequest,
) -> Result<SessionPair, ErrorKind> {
    let session_cookie_name = config
        .session_cookie_name
        .as_ref()
        .ok_or(ErrorKind::Msg("No session cookie name configured".into()))?;
    let session_value = req
        .cookie(&session_cookie_name)
        .ok_or(ErrorKind::Msg("No session cookie in request".into()))?;
    let session_id = Uuid::parse_str(session_value.value())
        .map_err(|_| ErrorKind::Msg("Invalid session cookie format".into()))?;

    let signature_cookie_name = config
        .session_cookie_sig_name
        .as_ref()
        .ok_or("Session cookie configured without signature name")?;
    let client_sig = req
        .cookie(&signature_cookie_name)
        .ok_or(ErrorKind::Msg("Session cookie without signature".into()))?;
    let client_sig = client_sig.value();

    Ok((session_id.to_hyphenated().to_string(), client_sig.into()))
}

fn verify_session_signature(
    config: &AppConfig,
    session_id: &str,
    signature: &str,
) -> Result<(), ErrorKind> {
    let session_cookie_name = config
        .session_cookie_name
        .as_ref()
        .ok_or(ErrorKind::Msg("No session cookie name configured".into()))?;
    let secret = config
        .session_secret
        .as_ref()
        .ok_or(ErrorKind::Msg("No session secret set".into()))?;

    verify_cookie_signature(session_cookie_name, session_id, signature, secret).and_then(|_| Ok(()))
}

pub fn verify_device_id_signature(
    config: &AppConfig,
    req: &HttpRequest,
) -> Result<Option<String>, ErrorKind> {
    let device_id_cookie_name = config
        .device_id_cookie_name
        .as_ref()
        .ok_or(ErrorKind::Msg("No device id cookie name configured".into()))?;
    let device_id_cookie_sig_name = config
        .device_id_cookie_sig_name
        .as_ref()
        .ok_or(ErrorKind::Msg("No device id cookie name configured".into()))?;
    let secret = config
        .session_secret
        .as_ref()
        .ok_or(ErrorKind::Msg("No device_id secret set".into()))?;

    if let Some(device_id_cookie) = req.cookie(device_id_cookie_name) {
        let device_id = device_id_cookie.into_owned();
        let signature = req.cookie(device_id_cookie_sig_name).unwrap().into_owned();

        verify_cookie_signature(
            device_id_cookie_name,
            device_id.value(),
            signature.value(),
            secret,
        )
        .and_then(|v| Ok(Some(String::from(v))))
    } else {
        Ok(None)
    }
}

fn verify_cookie_signature<'a>(
    cookie_name: &str,
    value: &'a str,
    signature: &str,
    secret: &str,
) -> Result<&'a str, ErrorKind> {
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, &secret.as_bytes());

    let sign = hmac::sign(&key, format!("{}={}", cookie_name, value).as_bytes());
    // https://github.com/tj/node-cookie-signature/blob/master/index.js#L23
    let generated_sig = base64::encode(&sign)
        .replace('/', "_")
        .replace('+', "-")
        .replace('=', "");

    if generated_sig == signature {
        Ok(value)
    } else {
        Err(ErrorKind::CookieInvalidSignature)
    }
}
