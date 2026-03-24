use log::debug;
use rastra_server::server::Server;
use rastra_server::utils::logger::Logger;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = rastra_config::setup_config();

    Logger::setup_logger(config.log_to_file, &config.logs_directory);
    debug!("Tokio runtime initialized");

    let server = Server::new(config);

    let server_ref = Arc::new(tokio::sync::Mutex::new(server));

    let server_clone = server_ref.clone();
    let server_handle = tokio::spawn(async move {
        server_clone.lock().await.run().await.unwrap();
    });

    let api_handle = tokio::spawn(async move {
        rastra_api::api::run().await.unwrap();
    });

    tokio::try_join!(server_handle, api_handle)?;

    Ok(())
}
