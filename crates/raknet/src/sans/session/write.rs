use crate::protocol::types::frame::Frame;
use crate::types::priority::RakPriority;
use std::net::SocketAddr;
use std::time::SystemTime;

pub enum Win {
    Frame(Frame, RakPriority, SystemTime),
}

pub enum Wout {
    Datagram(Vec<u8>, SocketAddr),
}
