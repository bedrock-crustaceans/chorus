use crate::protocol::codec::RakCodec;
use crate::protocol::packets::incompatible_protocol::IncompatibleProtocol;
use crate::protocol::packets::open_connection_reply_1::OpenConnectionReply1;
use crate::protocol::packets::open_connection_reply_2::OpenConnectionReply2;
use crate::protocol::packets::open_connection_request_1::OpenConnectionRequest1;
use crate::protocol::packets::open_connection_request_2::OpenConnectionRequest2;
use crate::protocol::packets::unconnected_ping::UnconnectedPing;
use crate::protocol::packets::unconnected_pong::UnconnectedPong;
use crate::server::config::RakServerConfig;
use crate::session::RakSession;
use crate::session::event::RakSessionEvent;
use crate::util::constants::{PROTOCOL, UDP_HEADER_SIZE};
use crate::util::flags::VALID;
use crate::util::packet_id;
use crate::util::socket_addr::get_overhead;
use dashmap::DashMap;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::debug;

pub struct RakServerInner {
    addr: SocketAddr,
    pub config: RakServerConfig,

    sessions: Arc<DashMap<SocketAddr, RakSession>>,

    packet_tx: UnboundedSender<(Vec<u8>, SocketAddr)>,
    event_tx: UnboundedSender<RakSessionEvent>,
}

impl RakServerInner {
    pub fn new(config: RakServerConfig, addr: SocketAddr, packet_tx: UnboundedSender<(Vec<u8>, SocketAddr)>, event_tx: UnboundedSender<RakSessionEvent>) -> Self {
        Self {
            config,
            addr,
            sessions: Arc::new(DashMap::new()),
            packet_tx,
            event_tx,
        }
    }

    pub async fn handle(&self, buf: &[u8], addr: SocketAddr) {
        if let Some(&header) = buf.first() {
            match header & VALID {
                0 => self.handle_offline(buf, addr).await,
                _ => {
                    if let Some(s) = self.sessions.get(&addr) {
                        _ = s.inbound(buf.to_vec());
                    }
                }
            }
        }
    }

    async fn handle_offline(&self, buf: &[u8], addr: SocketAddr) {
        if let Some(&id) = buf.first() {
            let mut cursor = Cursor::new(buf);
            match id {
                packet_id::UNCONNECTED_PING => self.handle_unconnected_ping(&mut cursor, addr).await,
                packet_id::OPEN_CONNECTION_REQUEST_1 => self.handle_open_connection_request_1(&mut cursor, addr).await,
                packet_id::OPEN_CONNECTION_REQUEST_2 => self.handle_open_connection_request_2(&mut cursor, addr).await,

                _ => debug!("received unknown offline packet from {}, id: {:#04X}", addr, id),
            }
        }
    }

    async fn handle_unconnected_ping(&self, cursor: &mut Cursor<&[u8]>, addr: SocketAddr) {
        let Ok(ping) = UnconnectedPing::deserialize(cursor) else {
            return debug!("failed to deserialize UnconnectedPing from {}", addr);
        };

        let pong = UnconnectedPong {
            timestamp: ping.timestamp,
            guid: self.config.guid,
            message: self.config.message.clone(),
        };

        let mut buf = Vec::with_capacity(UnconnectedPong::size_hint(&pong));
        UnconnectedPong::serialize(&pong, &mut buf).unwrap();

        self.send((buf, addr));
    }

    async fn handle_open_connection_request_1(&self, cursor: &mut Cursor<&[u8]>, addr: SocketAddr) {
        let Ok(request) = OpenConnectionRequest1::deserialize(cursor) else {
            return debug!("failed to deserialize OpenConnectionRequest1 from {}", addr);
        };

        let req_protocol = request.protocol;
        if req_protocol != PROTOCOL {
            let incompatible = IncompatibleProtocol {
                protocol: PROTOCOL,
                guid: self.config.guid,
            };

            debug!("refusing connection from {} due to incompatible protocol {}, expected {}", addr, req_protocol, PROTOCOL);

            let mut buf = Vec::with_capacity(IncompatibleProtocol::size_hint(&incompatible));
            IncompatibleProtocol::serialize(&incompatible, &mut buf).unwrap();

            self.send((buf, addr));

            return;
        }

        let reply = OpenConnectionReply1 {
            guid: self.config.guid,
            cookie: None,
            mtu: (request.mtu + UDP_HEADER_SIZE + get_overhead(&addr)).clamp(self.config.min_mtu_size, self.config.max_mtu_size),
        };

        let mut buf = Vec::with_capacity(OpenConnectionReply1::size_hint(&reply));
        OpenConnectionReply1::serialize(&reply, &mut buf).unwrap();

        self.send((buf, addr));
    }

    async fn handle_open_connection_request_2(&self, cursor: &mut Cursor<&[u8]>, addr: SocketAddr) {
        let Ok(request) = OpenConnectionRequest2::deserialize(cursor) else {
            return debug!("failed to deserialize OpenConnectionRequest2 from {}", addr);
        };

        if request.addr.port() != self.addr.port() {
            return debug!("refusing connection from {} due to port mismatch", addr);
        }

        let mtu = request.mtu;

        if !(self.config.min_mtu_size..=self.config.max_mtu_size).contains(&mtu) {
            return debug!("refusing connection from {} due to invalid mtu size", addr);
        }

        if self.sessions.contains_key(&addr) {
            return debug!("refusing connection from {} due to existing connection", addr);
        }

        debug!("establishing connection from {} with mtu size of {}", addr, mtu);

        let reply = OpenConnectionReply2::new(self.config.guid, addr, mtu, false);

        let mut buf = Vec::with_capacity(OpenConnectionReply2::size_hint(&reply));
        OpenConnectionReply2::serialize(&reply, &mut buf).unwrap();

        self.send((buf, addr));

        self.sessions.insert(addr, RakSession::new(self.event_tx.clone(), addr, request.client, request.mtu, |_| ()));
    }

    fn send(&self, packet: (Vec<u8>, SocketAddr)) {
        _ = self.packet_tx.send(packet);
    }

    pub async fn run_update_loop(self: Arc<Self>, mut packet_rx: UnboundedReceiver<(Vec<u8>, SocketAddr)>, mut event_rx: UnboundedReceiver<RakSessionEvent>) {
        let socket = UdpSocket::bind(self.addr).await.unwrap();

        let mut buf = vec![0u8; self.config.max_mtu_size as usize];

        loop {
            tokio::select! {
                recv = socket.recv_from(&mut buf) => {
                    if let Ok((len, addr)) = recv {
                        self.handle(&buf[..len], addr).await;
                    }
                }
                Some(packet) = packet_rx.recv() => {
                    socket.send_to(&packet.0, &packet.1).await.unwrap();
                }
                Some(event) = event_rx.recv() => {
                    match event {
                        RakSessionEvent::Disconnected(addr) => {
                            self.sessions.remove(&addr);
                        }
                        RakSessionEvent::Inbound(buf, addr) => {
                            if let Some(session) = self.sessions.get_mut(&addr) &&
                                let Some(&b) = buf.first() {
                                    let mut cursor = Cursor::new(buf.as_slice());
                                    match b {
                                        packet_id::CONNECTION_REQUEST => session.handle_connection_request(&mut cursor).await,
                                        packet_id::NEW_INCOMING_CONNECTION => session.handle_new_incoming_connection(&mut cursor).await,
                                        _ => debug!("packet from {}, id: {:#04X}", addr, b),
                                    }
                                }
                        }
                        RakSessionEvent::Outbound(buf, addr) => {
                            socket.send_to(&buf, addr).await.unwrap();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
