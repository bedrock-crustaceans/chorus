use crate::session::RakSessionId;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum RakSessionOutput {
    Packet(Vec<u8>),
    Datagram(Vec<u8>, SocketAddr),
    Disconnected(RakSessionId),
    Timeout(Duration),
}
