mod assets;
mod bulk;
mod hpf;
mod response_type;
mod responses;
mod serialization;
mod server;
mod service_info;
mod show_resource;

pub async fn serve() -> std::io::Result<()> {
    server::serve().await
}
