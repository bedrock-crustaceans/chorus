use crate::sans::session::{RakSession, RakSessionId};

pub enum Ein {
    Disconnected(RakSessionId),
}

pub enum Eout {
    Connected(RakSessionId, RakSession),
}
