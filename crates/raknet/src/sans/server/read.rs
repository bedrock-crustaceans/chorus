use crate::sans::session::RakSessionId;
use std::net::SocketAddr;
use std::time::SystemTime;

pub enum Rin {
    Datagram(Vec<u8>, SocketAddr, SystemTime),
}

pub enum Rout {
    Datagram(Vec<u8>, RakSessionId),
}
