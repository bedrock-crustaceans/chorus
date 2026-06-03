use raknet::prelude::Sans;
mod state;

use crate::server::state::Shutdown;
use crate::session::RakSession;
use raknet::prelude::{RakServer as RakServerIntl, RakServerConfig, RakServerInput, RakServerOutput, RakSessionId, RakSessionInput};
use state::{Initialized, Running};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tracing::debug;

pub struct RakServer<S> {
    state: S,
}

impl RakServer<Initialized> {
    pub fn new(addr: SocketAddr) -> Self {
        RakServer {
            state: Initialized {
                addr,
                config: RakServerConfig::default(),
            },
        }
    }

    pub fn config(&self) -> &RakServerConfig {
        &self.state.config
    }

    pub fn addr(&self) -> SocketAddr {
        self.state.addr
    }

    pub fn config_mut(&mut self) -> &mut RakServerConfig {
        &mut self.state.config
    }

    pub fn addr_mut(&mut self) -> &mut SocketAddr {
        &mut self.state.addr
    }

    pub fn start(self) -> RakServer<Running> {
        let Initialized { config, addr } = self.state;

        let (session_tx, session_rx) = unbounded_channel();

        let handle = tokio::spawn(async move {
            let mut sessions: HashMap<RakSessionId, UnboundedSender<RakSessionInput>> = HashMap::new();

            let socket = UdpSocket::bind(addr).await.unwrap();
            let mut buf = vec![0u8; config.max_mtu_size as usize];
            let mut server = RakServerIntl::new(config, addr);

            let (tx, mut rx) = unbounded_channel::<(Vec<u8>, SocketAddr)>();

            loop {
                tokio::select! {
                    Ok((len, addr)) = socket.recv_from(&mut buf) => {
                        let now = SystemTime::now();

                        server.handle(RakServerInput::Datagram(buf[..len].to_vec(), addr, now)).unwrap();
                    }
                    Some((buf, addr)) = rx.recv() => {
                        socket.send_to(&buf, addr).await.unwrap();
                    }
                }

                while let Some(msg) = server.poll() {
                    match msg {
                        RakServerOutput::SocketDatagram(buf, addr) => {
                            socket.send_to(&buf, addr).await.unwrap();
                        }
                        RakServerOutput::SessionDatagram(buf, id) => {
                            if let Some(session) = sessions.get_mut(&id) {
                                let now = SystemTime::now();

                                session.send(RakSessionInput::Datagram(buf, now)).unwrap();
                            }
                        }
                        RakServerOutput::SessionConnected(session) => {
                            let id = session.id;

                            debug!("session {:?} connected", id);

                            let (session, tx) = RakSession::spawn(*session, tx.clone());

                            sessions.insert(id, tx);

                            session_tx.send(session).unwrap();
                        }
                    }
                }
            }
        });

        RakServer {
            state: Running { handle, session_rx },
        }
    }
}

impl RakServer<Running> {
    pub fn stop(self) -> RakServer<Shutdown> {
        // TODO
        self.state.handle.abort();

        RakServer { state: Shutdown {} }
    }

    pub async fn recv(&mut self) -> Option<RakSession> {
        self.state.session_rx.recv().await
    }

    pub fn try_recv(&mut self) -> Option<RakSession> {
        self.state.session_rx.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::pending;

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

        let mut server = RakServer::new("127.0.0.1:19132".parse().unwrap());

        let config = server.config_mut();
        config.guid = 123456789;
        config.message = b"MCPE;Chorus;0;1.0.0;0;-1;123456789;Chorus;Survival".to_vec();

        let mut server = server.start();

        let mut sessions = Vec::new();

        loop {
            tokio::select! {
                Some(recv) = server.recv() => {
                    sessions.push(recv);
                    debug!("received session")
                }
            }
        }
    }
}
