use std::error::Error;
use crate::network::network::Network;
use log::{error, info};
use rastra_config::RAstraConfig;
use std::sync::Arc;
use std::time::{Duration, Instant};
use chrono::Utc;
use tokio::time::sleep;
use rastra_api::singleton;
use rastra_utils::rolling_float_average::RollingFloatAverage;

pub struct Server {
    pub config: Arc<RAstraConfig>,
    pub network: Option<Network>,

    is_running: bool,

    pub tick: i64,
    next_tick_ms: i64,

    tick_min: f64,
    usage_max: f64,

    tick_avg: RollingFloatAverage,
    usage_avg: RollingFloatAverage,
}

impl Server {
    pub fn new(config: RAstraConfig) -> Self {
        Self {
            config: Arc::new(config),
            network: None,

            is_running: true,

            tick: 0,
            next_tick_ms: Utc::now().timestamp_millis(),

            tick_min: 20.0,
            usage_max: 0.0,

            tick_avg: RollingFloatAverage::new(20),
            usage_avg: RollingFloatAverage::new(20)
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

        while self.is_running {
            if let Err(err) = self.tick().await {
                error!("{}", err);
                return Ok(());
            }

            let next_ms = self.next_tick_ms * 1000;
            let current_ms = Utc::now().timestamp_micros();

            if next_ms - 100 > current_ms {
                let allocated = next_ms - current_ms - 1000;
                if allocated > 0 {
                    sleep(Duration::from_micros(allocated as u64)).await
                }
            }
        }

        Ok(())
    }

    pub async fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        let tick_start = Utc::now().timestamp_millis();
        let tick_start_nano = Instant::now();

        self.tick += 1;

        let tick_elapsed_nano = tick_start_nano.elapsed().as_nanos();
        let tick = f64::min(
            20.0,
            1_000_000_000.0 / f64::max(1_000_000.0, tick_elapsed_nano as f64),
        );
        let usage = f64::min(1.0, tick_elapsed_nano as f64 / 50_000_000.0);

        if self.usage_max < usage {
            self.usage_max = usage;
        }

        if self.tick_min > tick {
            self.tick_min = tick;
        }

        self.tick_avg.add(tick);
        self.usage_avg.add(usage);

        if (self.next_tick_ms - tick_start) < -1000 {
            self.next_tick_ms = tick_start
        } else {
            self.next_tick_ms += 50
        }

        Ok(())
    }
}

singleton!(SERVER_INSTANCE: Server);