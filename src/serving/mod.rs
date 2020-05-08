mod bulk;
mod serialization;
mod server;
mod show_resource;

pub async fn serve() -> std::io::Result<()> {
    server::serve().await
}