use crate::network::handler::login_handler::LoginHandler;
use bedrockrs::network::listener::Listener;
use rastra_config::RAstraConfig;
use std::sync::Arc;
use crate::network::{GAME_VERSION, PROTOCOL_VERSION, RAK_VERSION};

pub struct Network {
    pub config: Arc<RAstraConfig>,
    listener: Listener,
}

impl Network {
    pub async fn new(config: Arc<RAstraConfig>) -> anyhow::Result<Self> {
        let addr = format!("{}:{}", config.ip, config.port).parse()?;

        let listener = Listener::new_raknet(
            addr,
            config.motd.to_string(),
            config.sub_motd.to_string(),
            GAME_VERSION.to_string(),
            PROTOCOL_VERSION,
            RAK_VERSION,
            config.max_players,
            0,
            false,
        )
        .await?;

        Ok(Self { config, listener })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.listener.start().await?;

        loop {
            let conn = self.listener.accept().await?;

            let _network_config = self.config.clone();
            tokio::spawn(async move {
                LoginHandler::handle(conn).await;
            });
        }
    }
}
