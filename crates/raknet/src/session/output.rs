use crate::session::RakSessionId;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum RakSessionOutput {
    Packet(Box<[u8]>),
    Datagram(Box<[u8]>, SocketAddr),
    Disconnected(RakSessionId),
    Timeout(Duration),
}
