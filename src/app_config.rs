use std::env;

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct AppConfig {
    /// The address the server should listen to
    pub binding: String,
    /// The timeout for data requests
    pub data_server_timeout: u64,
    /// The url of the server to retrieve the data from
    pub data_server_url: String,
    /// Enable to allow write commands via the HTTP interface
    pub enable_unsafe_methods: bool,
    /// The port the server should listen to
    pub port: String,
    /// Session cookie name to check for
    pub session_cookie_name: Option<String>,
    /// The address of cookie-to-authorization conversion service
    pub session_cookie_service_addr: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            binding: env::var("BINDING").unwrap_or("0.0.0.0".into()),
            enable_unsafe_methods: env::var("ENABLE_UNSAFE_METHODS")
                .and_then(|v| Ok(v == "true".to_string()))
                .unwrap_or_else(|_| false),
            port: env::var("PORT").unwrap_or("3030".into()),
            data_server_timeout: env::var("PROXY_TIMEOUT")
                .unwrap_or("20".into())
                .parse::<u64>()
                .unwrap(),
            data_server_url: env::var("ARGU_API_URL").expect("No data server url set"),
            session_cookie_name: env::var("SESSION_COOKIE_NAME").ok(),
            session_cookie_service_addr: env::var("SESSION_COOKIE_SERVICE_ADDR").ok(),
        }
    }
}
