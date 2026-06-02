use std::net::SocketAddr;
use std::time::SystemTime;

pub enum RakServerInput {
    Datagram(Vec<u8>, SocketAddr, SystemTime),
}
