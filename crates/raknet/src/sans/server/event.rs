use crate::sans::session::RakSessionId;

pub enum Ein {
    Disconnected(RakSessionId),
}