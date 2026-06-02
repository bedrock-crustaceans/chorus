use crate::sans::session::{RakSession, RakSessionId};
use std::net::SocketAddr;
use std::time::SystemTime;

pub enum Rin {
    Datagram(Vec<u8>, SocketAddr, SystemTime),
}

pub enum Rout {
    SocketDatagram(Vec<u8>, SocketAddr),
    SessionDatagram(Vec<u8>, RakSessionId),
    SessionConnected(RakSession),
}
