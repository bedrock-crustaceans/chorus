use raknet::sans::server::RakServer;
use raknet::sans::server::read::{Rin as ServerRin, Rout as ServerRout};
use raknet::sans::session::read::Rin as SessionRin;
use raknet::sans::session::{RakSession, RakSessionId};
use raknet::server::config::RakServerConfig;
use sansio::Protocol;
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

                        server.handle_read(ServerRin::Datagram(buf[..len].to_vec(), addr, now)).unwrap();

                        while let Some(msg) = server.poll_read() {
                            match msg {
                                ServerRout::SocketDatagram(buf, addr) => {
                                    socket.send_to(&buf, addr).await.unwrap();
                                },
                                ServerRout::SessionDatagram(buf, id) => {
                                    if let Some(session) = sessions.get_mut(&id) {
                                        let now = SystemTime::now();

                                        session.handle_read(SessionRin::Datagram(buf, now)).unwrap();
                                    }
                                },
                                ServerRout::SessionConnected(session) => {
                                    sessions.insert(session.id, session);
                                }
                            }
                        }
                    }
                }
            }
        });

        Self { handle }
    }

    pub async fn start(self) {}
}
