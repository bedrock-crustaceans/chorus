#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum RakSessionState {
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
}
