pub mod event;
pub mod read;
pub mod write;

use crate::protocol::codec::RakCodec;
use crate::protocol::packets::connection_request::ConnectionRequest;
use crate::protocol::packets::connection_request_accepted::ConnectionRequestAccepted;
use crate::protocol::packets::incompatible_protocol::IncompatibleProtocol;
use crate::protocol::packets::new_incoming_connection::NewIncomingConnection;
use crate::protocol::packets::open_connection_reply_1::OpenConnectionReply1;
use crate::protocol::packets::open_connection_reply_2::OpenConnectionReply2;
use crate::protocol::packets::open_connection_request_1::OpenConnectionRequest1;
use crate::protocol::packets::open_connection_request_2::OpenConnectionRequest2;
use crate::protocol::packets::unconnected_ping::UnconnectedPing;
use crate::protocol::packets::unconnected_pong::UnconnectedPong;
use crate::sans::server::read::{Rin, Rout};
use crate::sans::session::{RakSession, RakSessionId};
use crate::server::config::RakServerConfig;
use crate::session::state::RakSessionState;
use crate::types::priority::RakPriority;
use crate::types::reliability::RakReliability;
use crate::util::socket_addr::get_overhead;
use crate::util::{constants, flags, packet_id};
use sansio::Protocol;
use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

pub struct RakServer {
    addr: SocketAddr,
    config: RakServerConfig,

    session_id: RakSessionId,
    session_map: HashMap<SocketAddr, RakSessionId>,
    session_temp: HashMap<SocketAddr, RakSession>,

    rout: VecDeque<Rout>,
}

impl Protocol<Rin, (), ()> for RakServer {
    type Rout = Rout;
    type Wout = ();
    type Eout = ();
    type Error = ();
    type Time = ();

    fn handle_read(&mut self, msg: Rin) -> Result<(), Self::Error> {
        match msg {
            Rin::Datagram(buf, addr, now) => {
                let Some(&header) = buf.first() else {
                    return Ok(());
                };

                match header & flags::VALID {
                    0 => self.read_offline(&buf, addr),
                    _ => self.read_online(buf, addr, now),
                }
            }
        }
        Ok(())
    }

    fn poll_read(&mut self) -> Option<Self::Rout> {
        self.rout.pop_front()
    }

    fn handle_write(&mut self, _msg: ()) -> Result<(), Self::Error> {
        Ok(())
    }

    fn poll_write(&mut self) -> Option<Self::Wout> {
        None
    }
}

impl RakServer {
    pub fn new(config: RakServerConfig, addr: SocketAddr) -> Self {
        Self {
            config,
            addr,

            session_id: RakSessionId(0),
            session_map: HashMap::new(),
            session_temp: HashMap::new(),

            rout: VecDeque::new(),
        }
    }

    fn read_offline(&mut self, buf: &[u8], addr: SocketAddr) {
        if let Some(&id) = buf.first() {
            let mut cursor = Cursor::new(buf);
            match id {
                packet_id::UNCONNECTED_PING => self.read_unconnected_ping(&mut cursor, addr),
                packet_id::OPEN_CONNECTION_REQUEST_1 => self.read_open_connection_request_1(&mut cursor, addr),
                packet_id::OPEN_CONNECTION_REQUEST_2 => self.read_open_connection_request_2(&mut cursor, addr),

                _ => debug!("received unknown offline packet from {}, id: {:#04X}", addr, id),
            }
        }
    }

    fn read_online(&mut self, buf: Vec<u8>, addr: SocketAddr, now: SystemTime) {
        if self.session_temp.contains_key(&addr)
            && let Some(&b) = buf.first()
        {
            let mut cursor = Cursor::new(buf.as_slice());
            match b {
                packet_id::CONNECTION_REQUEST => return self.read_connection_request(addr, &mut cursor, now),
                packet_id::NEW_INCOMING_CONNECTION => return self.read_new_incoming_connection(addr, &mut cursor),
                _ => {}
            }
        }

        if let Some(&id) = self.session_map.get(&addr) {
            self.rout.push_back(Rout::SessionDatagram(buf, id));
        }
    }

    fn read_unconnected_ping(&mut self, cursor: &mut Cursor<&[u8]>, addr: SocketAddr) {
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

        self.rout.push_back(Rout::SocketDatagram(buf, addr));
    }

    fn read_open_connection_request_1(&mut self, cursor: &mut Cursor<&[u8]>, addr: SocketAddr) {
        let Ok(request) = OpenConnectionRequest1::deserialize(cursor) else {
            return debug!("failed to deserialize OpenConnectionRequest1 from {}", addr);
        };

        let req_protocol = request.protocol;
        if req_protocol != constants::PROTOCOL {
            let incompatible = IncompatibleProtocol {
                protocol: constants::PROTOCOL,
                guid: self.config.guid,
            };

            debug!("refusing connection from {} due to incompatible protocol {}, expected {}", addr, req_protocol, constants::PROTOCOL);

            let mut buf = Vec::with_capacity(IncompatibleProtocol::size_hint(&incompatible));
            IncompatibleProtocol::serialize(&incompatible, &mut buf).unwrap();

            self.rout.push_back(Rout::SocketDatagram(buf, addr));

            return;
        }

        let reply = OpenConnectionReply1 {
            guid: self.config.guid,
            cookie: None,
            mtu: (request.mtu + constants::UDP_HEADER_SIZE + get_overhead(&addr)).clamp(self.config.min_mtu_size, self.config.max_mtu_size),
        };

        let mut buf = Vec::with_capacity(OpenConnectionReply1::size_hint(&reply));
        OpenConnectionReply1::serialize(&reply, &mut buf).unwrap();

        self.rout.push_back(Rout::SocketDatagram(buf, addr));
    }

    fn read_open_connection_request_2(&mut self, cursor: &mut Cursor<&[u8]>, addr: SocketAddr) {
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

        if self.session_map.contains_key(&addr) {
            return debug!("refusing connection from {} due to existing connection", addr);
        }

        debug!("establishing connection from {} with mtu size of {}", addr, mtu);

        let reply = OpenConnectionReply2::new(self.config.guid, addr, mtu, false);

        let mut buf = Vec::with_capacity(OpenConnectionReply2::size_hint(&reply));
        OpenConnectionReply2::serialize(&reply, &mut buf).unwrap();

        self.rout.push_back(Rout::SocketDatagram(buf, addr));

        let id = self.session_id;
        self.session_id.0 += 1;

        self.session_map.insert(addr, id);
        self.session_temp.insert(addr, RakSession::new(id, addr, request.client, request.mtu, |_| ()));
    }

    fn read_connection_request(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>, now: SystemTime) {
        let Ok(request) = ConnectionRequest::deserialize(buf) else {
            return debug!("failed to deserialize ConnectionRequest from {}", self.addr);
        };

        let Some(session) = self.session_temp.get_mut(&addr) else {
            return debug!("unexpected ConnectionRequest from {}", self.addr);
        };

        debug!("handling connection request from {}", self.addr);

        let accepted = ConnectionRequestAccepted {
            client_address: self.addr,
            system_index: 0,
            system_addresses: vec![],
            request_timestamp: request.client_timestamp,
            timestamp: now.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
        };

        let mut buf = Vec::with_capacity(ConnectionRequestAccepted::size_hint(&accepted));
        ConnectionRequestAccepted::serialize(&accepted, &mut buf).unwrap();

        session.send(buf, RakReliability::ReliableOrdered, RakPriority::Normal, now);
    }

    fn read_new_incoming_connection(&mut self, addr: SocketAddr, buf: &mut Cursor<&[u8]>) {
        let Ok(_) = NewIncomingConnection::deserialize(buf) else {
            return debug!("failed to deserialize NewIncomingConnection from {}", addr);
        };

        let Some(mut session) = self.session_temp.remove(&addr) else {
            return debug!("unexpected NewIncomingConnection from {}", addr);
        };

        session.state = RakSessionState::Connected;

        self.rout.push_back(Rout::SessionConnected(session))
    }
}
