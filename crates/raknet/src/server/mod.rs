use crate::server::config::RakServerConfig;
use crate::server::inner::RakServerInner;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

mod config;
mod inner;

#[derive(Clone)]
pub struct RakServer {
    inner: Arc<RakServerInner>,
}

impl RakServer {
    pub async fn start<F>(addr: SocketAddr, conf: F) -> Self
    where
        F: FnOnce(&mut RakServerConfig),
    {
        let mut config = RakServerConfig::default();
        conf(&mut config);

        let (packet_tx, packet_rx) = unbounded_channel();
        let (event_tx, event_rx) = unbounded_channel();

        let inner = Arc::new(RakServerInner::new(config, addr, packet_tx, event_tx));

        tokio::spawn({
            let inner = inner.clone();
            async move {
                inner.run_update_loop(packet_rx, event_rx).await;
            }
        });

        Self { inner }
    }

    pub async fn stop(&mut self) {
        // TODO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::pending;
    use std::net::{Ipv4Addr, SocketAddrV4};

    #[tokio::test]
    #[ignore]
    async fn rak_server() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_target(true)
            .with_thread_ids(true)
            .with_line_number(true)
            .with_test_writer()
            .compact()
            .try_init();

        RakServer::start(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 19132)), |config| {
            config.message = {
                let mut buf = Vec::new();

                let str = ["MCPE", "Chorus", "0", "1.0.0", "0", "-1", "123456789", "Chorus", "Survival"].join(";");

                buf.extend(str.as_bytes());
                buf
            };
            config.guid = 123456789;
        })
        .await;

        pending().await
    }
}
