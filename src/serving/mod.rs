mod assets;
mod bulk;
mod response_type;
mod responses;
mod serialization;
mod server;
mod show_resource;
mod welcome;

pub async fn serve() -> std::io::Result<()> {
    server::serve().await
}
