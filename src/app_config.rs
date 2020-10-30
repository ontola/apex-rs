use std::env;
use std::str::FromStr;

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
    pub cluster_config: ClusterConfig,
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

#[derive(Clone, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct ClusterConfig {
    pub cluster_domain: String,
    pub cluster_url_base: String,
    pub default_backend_service_name: String,
    pub default_service_port: Option<u16>,
    pub default_service_proto: String,
    pub namespace: String,
    pub svc_dns_prefix: String,
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
                    let postgresql_username =
                        env::var("POSTGRESQL_USERNAME").unwrap_or("postgres".into());
                    let postgresql_address =
                        env::var("POSTGRESQL_ADDRESS").unwrap_or("localhost".into());
                    let postfix = env::var("APEX_POSTGRESQL_POSTFIX").unwrap_or("".into());

                    let connstr = format!(
                        "postgres://{}:{}@{}/{}{}",
                        postgresql_username,
                        postgresql_password,
                        postgresql_address,
                        database_name,
                        postfix
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
                .split("?")
                .nth(0)
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
            cluster_config: ClusterConfig::default(),
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

impl AppConfig {
    pub fn create_redis_consumer(&self) -> redis::RedisResult<redis::Connection> {
        let client = redis::Client::open(self.redis_url.clone())?;
        client.get_connection()
    }
}

fn dot_prefix(value: &str) -> String {
    if value.len() > 0 {
        format!(".{}", value)
    } else {
        String::from(value)
    }
}

impl Default for ClusterConfig {
    fn default() -> ClusterConfig {
        let default_port = 3000u16;
        let default_service_port = env::var("DEFAULT_SERVICE_PORT").map_or_else(
            |_| default_port,
            |port| u16::from_str(&port).unwrap_or(default_port),
        );

        let namespace = env::var("NAMESPACE").unwrap_or("".into());
        let svc_dns_prefix = env::var("SERVICE_DNS_PREFIX").unwrap_or("svc".into());
        let svc_dns_prefix_insert = dot_prefix(&svc_dns_prefix);
        let default_cluster = String::from("cluster.local");
        let mut cluster_domain = env::var("CLUSTER_DOMAIN").unwrap_or(default_cluster.clone());
        if cluster_domain.len() == 0 {
            cluster_domain = default_cluster;
        }
        let fallback = format!(
            "{}{}.{}",
            dot_prefix(&namespace),
            svc_dns_prefix_insert,
            cluster_domain
        );

        ClusterConfig {
            cluster_domain,
            cluster_url_base: env::var("CLUSTER_URL_BASE").unwrap_or(fallback),
            default_backend_service_name: env::var("DEFAULT_BACKEND_SVC_NAME")
                .unwrap_or("argu".into()),
            default_service_port: Some(default_service_port),
            default_service_proto: env::var("DEFAULT_SERVICE_PROTO").unwrap_or("http".into()),
            namespace,
            svc_dns_prefix,
        }
    }
}
