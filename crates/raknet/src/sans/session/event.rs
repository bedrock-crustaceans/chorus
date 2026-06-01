use crate::sans::session::RakSessionId;

pub enum Eout {
    Disconnected(RakSessionId),
}
