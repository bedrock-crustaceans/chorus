use crate::types::RakReliability;
use crate::types::priority::RakPriority;
use std::time::SystemTime;

#[derive(Clone, Debug)]
pub enum RakSessionInput {
    Datagram(Box<[u8]>, SystemTime),
    Send(Box<[u8]>, RakReliability, RakPriority, SystemTime),
    Timeout(SystemTime),
}
