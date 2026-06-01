use std::net::SocketAddr;

pub enum Eout {
    Connected(SocketAddr),
    Disconnected(SocketAddr),
}
