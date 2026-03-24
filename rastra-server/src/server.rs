use std::sync::Arc;
use log::info;
use rastra_config::RAstraConfig;
use crate::network::network::Network;

pub struct Server {
    pub config: Arc<RAstraConfig>,
    pub network: Option<Network>,
}

impl Server {
    pub fn new(config: RAstraConfig) -> Self {
        Self {
            config: Arc::new(config),
            network: None,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("Server starting on {}:{}", self.config.ip, self.config.port);

        let network = Network::new(self.config.clone()).await?;
        self.network = Some(network);

        info!("Server started successfully!");

        if let Some(network) = self.network.as_mut() {
            network.run().await?;
        }

        Ok(())
    }
}