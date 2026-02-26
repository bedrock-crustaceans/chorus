use log::debug;
use rastra_server::server::Server;
use rastra_server::utils::Logger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = rastra_config::setup_config();

    Logger::setup_logger(config.log_to_file, &config.logs_directory);

    debug!("Tokio runtime initialized");

    let server = Server { config };

    server.run().await.expect("Server crashed");

    rastra_api::api::run().await.expect("API crashed");

    Ok(())
}
