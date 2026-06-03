use raknet::prelude::{RakPriority, RakReliability};
use tokio::sync::oneshot::Sender;

pub enum RakSessionMsg {
    Send(Box<[u8]>, RakReliability, RakPriority),
    Close(Sender<()>),
    IsClosed(Sender<bool>),
}
