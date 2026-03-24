use bedrockrs::proto::listener::Listener;
use bedrockrs::proto::v800;
use rastra_config::RAstraConfig;
use crate::network::handler::login_handler::LoginHandler;
use std::sync::Arc;

pub struct Network {
    pub config: Arc<RAstraConfig>,
    listener: Listener,
}

impl Network {
    pub async fn new(config: Arc<RAstraConfig>) -> anyhow::Result<Self> {
        let addr = format!("{}:{}", config.ip, config.port).parse()?;

        let listener = Listener::new_raknet(
            config.motd.to_string(),
            config.sub_motd.to_string(),
            v800::info::GAME_VERSION.to_string(),
            v800::info::PROTOCOL_VERSION,
            config.max_players,
            0,
            addr,
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