use crate::session::{RakSession, RakSessionId};
use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub enum RakServerOutput {
    SocketDatagram(Vec<u8>, SocketAddr),
    SessionDatagram(Vec<u8>, RakSessionId),
    SessionConnected(RakSession),
}
