use raknet::sans::Sans;
use raknet::server::RakServer;
use raknet::server::config::RakServerConfig;
use raknet::server::input::RakServerInput;
use raknet::server::output::RakServerOutput;
use raknet::session::input::RakSessionInput;
use raknet::session::output::RakSessionOutput;
use raknet::session::{RakSession, RakSessionId};
use std::collections::HashMap;
use std::future::pending;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::{Duration, SystemTime};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep};
use tracing::debug;

pub struct RakServerTokio {
    handle: JoinHandle<()>,
}

impl RakServerTokio {
    pub fn new<F>(addr: SocketAddr, conf: F) -> Self
    where
        F: FnOnce(&mut RakServerConfig),
    {
        let mut config = RakServerConfig::default();
        conf(&mut config);

        let handle = tokio::spawn(async move {
            let mut sessions: HashMap<RakSessionId, UnboundedSender<RakSessionInput>> = HashMap::new();

            let socket = UdpSocket::bind(addr).await.unwrap();
            let mut buf = vec![0u8; config.max_mtu_size as usize];
            let mut server = RakServer::new(config, addr);

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
                            debug!("session {:?} connected", session.id);

                            sessions.insert(session.id, Self::spawn_session(tx.clone(), session));
                        }
                    }
                }
            }
        });

        Self { handle }
    }

    pub async fn start(self) {}

    fn spawn_session(datagram_tx: UnboundedSender<(Vec<u8>, SocketAddr)>, session: RakSession) -> UnboundedSender<RakSessionInput> {
        let (tx, rx) = unbounded_channel();

        tokio::spawn(async move {
            let mut rx = rx;
            let mut session = session;

            let timeout = sleep(Duration::ZERO);
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    Some(recv) = rx.recv() => {
                        session.handle(recv).unwrap();
                    }
                    _ = &mut timeout => {
                        let now = SystemTime::now();

                        session.handle(RakSessionInput::Timeout(now)).unwrap();
                    }
                }

                while let Some(msg) = session.poll() {
                    match msg {
                        RakSessionOutput::Timeout(when) => timeout.as_mut().reset(Instant::now() + when),
                        RakSessionOutput::Datagram(buf, addr) => {
                            datagram_tx.send((buf, addr)).unwrap();
                        }
                        RakSessionOutput::Packet(buf) => {
                            let Some(&b) = buf.first() else {
                                continue;
                            };
                            debug!("received packet 0x{:02X} from {}", b, session.addr)
                        }
                        RakSessionOutput::Disconnected(..) => return,
                    }
                }
            }
        });

        tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

        RakServerTokio::new(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 19132)), |config| {
            config.message = {
                let mut buf = Vec::new();

                let str = ["MCPE", "Chorus", "0", "1.0.0", "0", "-1", "123456789", "Chorus", "Survival"].join(";");

                buf.extend(str.as_bytes());
                buf
            };
            config.guid = 123456789;
        });

        pending().await
    }
}
