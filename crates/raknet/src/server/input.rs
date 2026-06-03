use std::net::SocketAddr;
use std::time::SystemTime;

pub enum RakServerInput {
    Datagram(Box<[u8]>, SocketAddr, SystemTime),
}
