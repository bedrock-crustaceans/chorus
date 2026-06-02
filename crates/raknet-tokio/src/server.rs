use raknet::sans::Sans;
use raknet::sans::server::RakServer;
use raknet::sans::server::input::RakServerInput;
use raknet::sans::server::output::RakServerOutput;
use raknet::sans::session::input::RakSessionInput;
use raknet::sans::session::{RakSession, RakSessionId};
use raknet::server::config::RakServerConfig;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;

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
            let mut sessions: HashMap<RakSessionId, RakSession> = HashMap::new();

            let socket = UdpSocket::bind(addr).await.unwrap();

            let mut buf = vec![0u8; config.max_mtu_size as usize];

            let mut server = RakServer::new(config, addr);

            loop {
                tokio::select! {
                    Ok((len, addr)) = socket.recv_from(&mut buf) => {
                        let now = SystemTime::now();

                        server.handle(RakServerInput::Datagram(buf[..len].to_vec(), addr, now)).unwrap();
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

                                session.handle(RakSessionInput::Datagram(buf, now)).unwrap();
                            }
                        }
                        RakServerOutput::SessionConnected(session) => {
                            sessions.insert(session.id, session);
                        }
                    }
                }
            }
        });

        Self { handle }
    }

    pub async fn start(self) {}
}
