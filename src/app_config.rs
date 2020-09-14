use std::env;

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct AppConfig {
    /// The address the server should listen to
    pub binding: String,
    /// OAuth client id
    pub client_id: Option<String>,
    /// OAuth client secret
    pub client_secret: Option<String>,
    /// The timeout for data requests
    pub data_server_timeout: u64,
    /// The url of the server to retrieve the data from
    pub data_server_url: String,
    /// Enable to allow write commands via the HTTP interface
    pub enable_unsafe_methods: bool,
    pub jwt_encryption_token: Option<String>,
    /// The port the server should listen to
    pub port: String,
    pub redis_url: String,
    /// Token used when no authentication was provided
    pub service_guest_token: Option<String>,
    /// Session cookie name to check for
    pub session_cookie_name: Option<String>,
    /// Session cookie signature name to check for
    pub session_cookie_sig_name: Option<String>,
    pub session_secret: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            binding: env::var("BINDING").unwrap_or("0.0.0.0".into()),
            client_id: env::var("ARGU_APP_ID")
                .or_else(|_| env::var("LIBRO_APP_ID"))
                .ok(),
            client_secret: env::var("ARGU_APP_SECRET")
                .or_else(|_| env::var("LIBRO_APP_SECRET"))
                .ok(),
            jwt_encryption_token: env::var("JWT_ENCRYPTION_TOKEN").ok(),
            enable_unsafe_methods: env::var("ENABLE_UNSAFE_METHODS")
                .and_then(|v| Ok(v == "true".to_string()))
                .unwrap_or_else(|_| false),
            port: env::var("PORT").unwrap_or("3030".into()),
            data_server_timeout: env::var("PROXY_TIMEOUT")
                .unwrap_or("20".into())
                .parse::<u64>()
                .unwrap(),
            data_server_url: env::var("ARGU_API_URL").expect("No data server url set"),
            redis_url: env::var("REDIS_URL").unwrap_or("redis://127.0.0.1/".into()),
            service_guest_token: env::var("SERVICE_GUEST_TOKEN").ok(),
            session_cookie_name: env::var("SESSION_COOKIE_NAME").ok(),
            session_cookie_sig_name: env::var("SESSION_COOKIE_SIGNATURE_NAME").ok(),
            session_secret: env::var("SESSION_SECRET").ok(),
        }
    }
}
