pub mod config;
pub mod congestion_controller;
pub mod event;
pub mod inner;
pub mod message;
pub mod state;

use crate::session::config::RakSessionConfig;
use crate::session::event::RakSessionEvent;
use crate::session::inner::RakSessionInner;
use crate::types::priority::RakPriority;
use crate::types::reliability::RakReliability;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

pub struct RakSession {
    inner: Arc<RakSessionInner>,
}

impl RakSession {
    pub fn new<F>(event_tx: UnboundedSender<RakSessionEvent>, addr: SocketAddr, guid: u64, mtu: u16, conf: F) -> Self
    where
        F: FnOnce(&mut RakSessionConfig),
    {
        let (msg_tx, msg_rx) = unbounded_channel();

        let inner = Arc::new(RakSessionInner::new(event_tx, msg_tx, addr, guid, mtu, conf));

        tokio::spawn({
            let inner = inner.clone();
            async move {
                inner.run_update_loop(msg_rx).await;
            }
        });

        Self { inner }
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.inner.get_addr()
    }

    pub fn send(&self, buf: Vec<u8>, reliability: RakReliability, priority: RakPriority) {
        self.inner.send(buf, reliability, priority);
    }

    pub fn inbound(&self, buf: Vec<u8>) {
        self.inner.inbound(buf);
    }

    pub async fn handle_connection_request(&self, buf: &mut Cursor<&[u8]>) {
        self.inner.handle_connection_request(buf).await;
    }

    pub async fn handle_new_incoming_connection(&self, buf: &mut Cursor<&[u8]>) {
        self.inner.handle_new_incoming_connection(buf).await;
    }
}
