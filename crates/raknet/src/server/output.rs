use crate::session::{RakSession, RakSessionId};
use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub enum RakServerOutput {
    SocketDatagram(Box<[u8]>, SocketAddr),
    SessionDatagram(Box<[u8]>, RakSessionId),
    SessionConnected(Box<RakSession>),
}
