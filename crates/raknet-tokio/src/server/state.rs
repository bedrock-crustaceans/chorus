use raknet::prelude::RakServerConfig;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

pub struct Initialized {
    pub(crate) addr: SocketAddr,
    pub(crate) config: RakServerConfig,
}

pub struct Running {
    pub(crate) handle: JoinHandle<()>,
}

pub struct Shutdown {}
