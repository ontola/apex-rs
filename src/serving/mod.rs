mod assets;
mod bulk;
mod hpf;
mod response_type;
mod responses;
pub(crate) mod serialization;
mod server;
mod service_info;
mod show_resource;
pub(crate) mod ua;
mod update;

pub async fn serve() -> std::io::Result<()> {
    server::serve().await
}
