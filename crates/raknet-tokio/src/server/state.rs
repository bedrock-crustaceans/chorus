use crate::server::msg::RakServerMsg;
use crate::session::RakSession;
use raknet::prelude::RakServerConfig;
use std::net::SocketAddr;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

pub struct Initialized {
    pub(crate) addr: SocketAddr,
    pub(crate) config: RakServerConfig,
}

pub struct Running {
    pub(crate) handle: JoinHandle<()>,
    pub(crate) session_rx: UnboundedReceiver<RakSession>,
    pub(crate) msg_tx: UnboundedSender<RakServerMsg>,
}

pub enum RakServerState {
    Initialized(Initialized),
    Running(Running),
    Shutdown,
}
