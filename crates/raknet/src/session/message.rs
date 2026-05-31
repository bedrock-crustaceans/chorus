use crate::protocol::types::frame::Frame;
use crate::types::priority::RakPriority;

pub enum RakSessionMessage {
    OutFrame(Frame, RakPriority),
    InBuf(Vec<u8>),
}
