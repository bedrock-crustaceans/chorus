use crate::protocol::types::frame::Frame;
use crate::types::priority::RakPriority;
use std::time::SystemTime;

#[derive(Clone, Debug)]
pub enum RakSessionInput {
    Datagram(Vec<u8>, SystemTime),
    SendFrame(Frame, RakPriority, SystemTime),
    Timeout(SystemTime),
}
