use raknet::prelude::Sans;
pub mod msg;
mod state;

use crate::server::msg::RakServerMsg;
use crate::server::state::RakServerState;
use crate::session::RakSession;
use raknet::prelude::{RakServer as RakServerIntl, RakServerConfig, RakServerInput, RakServerOutput, RakSessionId, RakSessionInput};
use state::{Initialized, Running};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tracing::debug;

pub struct RakServer {
    state: RakServerState,
}

impl RakServer {
    pub fn new(addr: SocketAddr) -> Self {
        RakServer {
            state: RakServerState::Initialized(Initialized {
                addr,
                config: RakServerConfig::default(),
            }),
        }
    }

    pub fn set_guid(&mut self, val: u64) {
        let RakServerState::Initialized(Initialized { config, .. }) = &mut self.state else {
            return;
        };

        config.guid = val;
    }

    pub fn set_message<T>(&mut self, val: T)
    where
        T: Into<Box<[u8]>>,
    {
        let buf = val.into();
        match &mut self.state {
            RakServerState::Initialized(Initialized { config, .. }) => {
                config.message = buf;
            }
            RakServerState::Running(Running { msg_tx, .. }) => {
                msg_tx.send(RakServerMsg::SetMessage(buf)).unwrap();
            }
            _ => {}
        }
    }

    pub fn set_max_connections(&mut self, val: usize) {
        match &mut self.state {
            RakServerState::Initialized(Initialized { config, .. }) => {
                config.max_connections = val;
            }
            RakServerState::Running(Running { msg_tx, .. }) => {
                msg_tx.send(RakServerMsg::SetMaxConnections(val)).unwrap();
            }
            _ => {}
        }
    }

    pub fn start(&mut self) {
        let RakServerState::Initialized(Initialized { config, addr }) = &self.state else {
            return;
        };

        let (session_tx, session_rx) = unbounded_channel();
        let (msg_tx, msg_rx) = unbounded_channel();

        let handle = tokio::spawn({
            let config = config.clone();
            let addr = addr.clone();

            async move {
                let mut msg_rx = msg_rx;

                let mut sessions: HashMap<RakSessionId, UnboundedSender<RakSessionInput>> = HashMap::new();

                let socket = UdpSocket::bind(addr).await.unwrap();
                let mut buf = vec![0u8; config.max_mtu_size as usize];
                let mut server = RakServerIntl::new(config, addr);

                let (dgram_tx, mut dgram_rx) = unbounded_channel::<(Box<[u8]>, SocketAddr)>();

                loop {
                    tokio::select! {
                        Ok((len, addr)) = socket.recv_from(&mut buf) => {
                            let now = SystemTime::now();

                            server.handle(RakServerInput::Datagram(buf[..len].into(), addr, now)).unwrap();
                        }
                        Some((buf, addr)) = dgram_rx.recv() => {
                            socket.send_to(buf.as_ref(), addr).await.unwrap();
                        }
                        Some(msg) = msg_rx.recv() => {
                            match msg {
                                RakServerMsg::SetMessage(msg) => {
                                    server.handle(RakServerInput::SetMessage(msg)).unwrap();
                                },
                                RakServerMsg::SetMaxConnections(n) => {
                                    server.handle(RakServerInput::SetMaxConnections(n)).unwrap();
                                }
                            }
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

                                let (session, tx) = RakSession::spawn(*session, dgram_tx.clone());

                                sessions.insert(id, tx);

                                session_tx.send(session).unwrap();
                            }
                        }
                    }
                }
            }
        });

        self.state = RakServerState::Running(Running { handle, session_rx, msg_tx });
    }
}

impl RakServer {
    pub fn stop(&mut self) {
        let RakServerState::Running(Running { handle, .. }) = &self.state else {
            return;
        };

        // TODO
        handle.abort();

        self.state = RakServerState::Shutdown;
    }

    pub async fn accept(&mut self) -> Option<RakSession> {
        let RakServerState::Running(Running { session_rx, .. }) = &mut self.state else {
            return None;
        };

        session_rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        server.set_guid(123456789);
        server.set_message(*b"MCPE;Chorus;0;1.0.0;0;-1;123456789;Chorus;Survival");

        server.start();

        let mut sessions = Vec::new();

        loop {
            tokio::select! {
                Some(session) = server.accept() => {
                    sessions.push(session);
                    debug!("received session")
                }
            }
        }
    }
}
