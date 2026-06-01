use std::net::SocketAddr;

pub enum Wout {
    Datagram(Vec<u8>, SocketAddr),
}
