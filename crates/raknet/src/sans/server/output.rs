use crate::sans::session::{RakSession, RakSessionId};
use std::net::SocketAddr;

pub enum RakServerOutput {
    SocketDatagram(Vec<u8>, SocketAddr),
    SessionDatagram(Vec<u8>, RakSessionId),
    SessionConnected(RakSession),
}
