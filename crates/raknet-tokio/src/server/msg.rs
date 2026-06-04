pub enum RakServerMsg {
    SetMessage(Box<[u8]>),
    SetMaxConnections(usize),
}
