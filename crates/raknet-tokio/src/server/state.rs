use crate::session::RakSession;
use raknet::prelude::RakServerConfig;
use std::net::SocketAddr;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;

pub struct Initialized {
    pub(crate) addr: SocketAddr,
    pub(crate) config: RakServerConfig,
}

pub struct Running {
    pub(crate) handle: JoinHandle<()>,
    pub(crate) session_rx: UnboundedReceiver<RakSession>,
}

pub struct Shutdown {}
