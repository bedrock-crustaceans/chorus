use crate::session::RakSessionId;

pub enum Ein {
    Disconnected(RakSessionId),
}
