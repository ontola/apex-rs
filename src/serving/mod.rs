mod assets;
mod bulk;
mod bulk_ctx;
mod health;
mod hpf;
mod metrics;
pub(crate) mod reporter;
mod request_headers;
mod response_type;
mod responses;
mod route;
pub(crate) mod serialization;
mod server;
mod service_info;
pub(crate) mod sessions;
mod show_resource;
pub(crate) mod timings;
pub(crate) mod ua;
mod update;

pub async fn serve() -> std::io::Result<()> {
    server::serve().await
}
