use bedrockrs::proto::connection::Connection;
use bedrockrs::proto::listener::Listener;
use bedrockrs::proto::v786;
use log::{debug, info};
use rastra_config::RAstraConfig;

pub struct Server {
    pub config: RAstraConfig,
}

impl Server {
    pub async fn run(&self) -> anyhow::Result<()> {
        let config = &self.config;

        let ip = &config.ip;
        let port = config.port;

        info!("Server starting on {}:{}", ip, port);

        let mut listener = Listener::new_raknet(
            config.motd.to_string(),
            config.sub_motd.to_string(),
            "1.21.70".to_string(),
            v786::info::PROTOCOL_VERSION,
            config.max_players,
            0, // TODO
            format!("{}:{}", ip, port).parse().unwrap(),
            false,
        )
        .await?;

        listener.start().await?;

        info!("Server started successfully!");

        loop {
            let conn = listener.accept().await?;

            tokio::spawn(async move {
                handle_conn(conn).await;
            });
        }

        async fn handle_conn(conn: Connection) {
            debug!(
                "Got connection from {:?}",
                conn.get_socket_addr().to_string()
            );
        }
    }
}
