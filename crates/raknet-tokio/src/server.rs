use std::collections::{HashMap, HashSet};
use sansio::Protocol;
use std::net::SocketAddr;
use std::time::SystemTime;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tracing::debug;
use raknet::server::config::RakServerConfig;
use raknet::sans::server::RakServer;
use raknet::sans::server::read::{Rin as ServerRin, Rout as ServerRout};
use raknet::sans::server::write::Wout as ServerWout;
use raknet::sans::server::event::Eout as ServerEout;
use raknet::sans::session::{RakSession, RakSessionId};
use raknet::sans::session::read::{Rin as SessionRin, Rout as SessionRout};
use raknet::sans::session::write::{Wout as SessionWout};
use raknet::sans::session::event::Eout as SessionEout;

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
                while let Ok((len, addr)) = socket.try_recv_from(&mut buf) {
                    let now = SystemTime::now();
                    
                    server.handle_read(ServerRin::Datagram(buf[..len].to_vec(), addr, now)).unwrap();
                }

                while let Some(msg) = server.poll_read() {
                    match msg {
                        ServerRout::Datagram(buf, id) => {
                            if let Some(session) = sessions.get_mut(&id) {
                                let now = SystemTime::now();
                                
                                session.handle_read(SessionRin::Datagram(buf, now)).unwrap();
                            }
                        }
                    }
                }

                while let Some(msg) = server.poll_write() {
                    match msg {
                        ServerWout::Datagram(buf, addr) => {
                            socket.send_to(&buf, addr).await.unwrap();
                        }
                    }
                }

                while let Some(msg) = server.poll_event() {
                    match msg {
                        ServerEout::Connected(id, session) => {
                            sessions.insert(id, session);
                        }
                    }
                }
                
                let mut disconnected = HashSet::new();
                for (_, session) in sessions.iter_mut() {
                    while let Some(msg) = session.poll_read() {
                        match msg { 
                            SessionRout::Datagram(buf) => {
                                if let Some(b) = buf.first() {
                                    debug!("packet from {:?} [{}], id: {:#04X}", session.id, session.addr, b)
                                }
                            }
                        }
                    }
                    
                    while let Some(msg) = session.poll_write() {
                        match msg {
                            SessionWout::Datagram(buf, addr) => {
                                socket.send_to(&buf, addr).await.unwrap();
                            }
                        }
                    }
                    
                    while let Some(msg) = session.poll_event() {
                        match msg { 
                            SessionEout::Disconnected(id) => {
                                disconnected.insert(id);
                            }
                        }
                    }
                }
                
                for id in disconnected {
                    sessions.remove(&id);
                }
            }
        });
        
        Self {
            handle
        }
    }
    
    pub async fn start(self) {
        
    }
}