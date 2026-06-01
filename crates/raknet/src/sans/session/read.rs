use std::time::SystemTime;

pub enum Rin {
    Datagram(Vec<u8>, SystemTime),
}

pub enum Rout {
    Datagram(Vec<u8>),
}
