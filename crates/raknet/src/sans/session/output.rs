use crate::sans::session::RakSessionId;
use std::net::SocketAddr;
use std::time::SystemTime;

pub enum RakSessionOutput {
    Packet(Vec<u8>),
    Datagram(Vec<u8>, SocketAddr),
    Disconnected(RakSessionId),
    Timeout(SystemTime),
}
