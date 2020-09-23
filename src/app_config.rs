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
    pub data_server_url: Option<String>,
    /// The connection string of the database
    pub database_url: Option<String>,
    /// The name of the database
    pub database_name: String,
    /// Controls whether persistent storage is used for data.
    pub disable_persistence: bool,
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
        let mut database_name = env::var("APEX_DATABASE_NAME").unwrap_or("apex_rs".into());
        let database_url = match env::var("DATABASE_URL") {
            Ok(url) => Some(url),
            Err(_) => match env::var("POSTGRESQL_PASSWORD") {
                Err(_) => {
                    warn!(target: "apex", "No DATABASE_URL nor POSTGRESQL_PASSWORD set");
                    None
                }
                Ok(postgresql_password) => {
                    warn!("No ");
                    let postgresql_username =
                        env::var("POSTGRESQL_USERNAME").unwrap_or("postgres".into());
                    let postgresql_address =
                        env::var("POSTGRESQL_ADDRESS").unwrap_or("localhost".into());
                    let connstr = format!(
                        "postgres://{}:{}@{}/{}",
                        postgresql_username, postgresql_password, postgresql_address, database_name
                    );

                    Some(connstr.into())
                }
            },
        };
        if let Some(database_url) = &database_url {
            database_name = database_url
                .clone()
                .split("/")
                .last()
                .expect("Couldn't determine database name")
                .into();
        }

        AppConfig {
            binding: env::var("BINDING").unwrap_or("0.0.0.0".into()),
            client_id: env::var("ARGU_APP_ID")
                .or_else(|_| env::var("LIBRO_APP_ID"))
                .ok(),
            client_secret: env::var("ARGU_APP_SECRET")
                .or_else(|_| env::var("LIBRO_APP_SECRET"))
                .ok(),
            data_server_timeout: env::var("PROXY_TIMEOUT")
                .unwrap_or("20".into())
                .parse::<u64>()
                .unwrap(),
            data_server_url: env::var("ARGU_API_URL").ok(),
            database_url,
            database_name,
            disable_persistence: env::var("DISABLE_PERSISTENCE")
                .and_then(|v| Ok(v == "true".to_string()))
                .unwrap_or_else(|_| false),
            enable_unsafe_methods: env::var("ENABLE_UNSAFE_METHODS")
                .and_then(|v| Ok(v == "true".to_string()))
                .unwrap_or_else(|_| false),
            jwt_encryption_token: env::var("JWT_ENCRYPTION_TOKEN").ok(),
            port: env::var("PORT").unwrap_or("3030".into()),
            redis_url: env::var("REDIS_URL").unwrap_or("redis://127.0.0.1/".into()),
            service_guest_token: env::var("SERVICE_GUEST_TOKEN").ok(),
            session_cookie_name: env::var("SESSION_COOKIE_NAME").ok(),
            session_cookie_sig_name: env::var("SESSION_COOKIE_SIGNATURE_NAME").ok(),
            session_secret: env::var("SESSION_SECRET").ok(),
        }
    }
}
