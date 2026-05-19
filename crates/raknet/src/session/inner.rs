use crate::protocol::codec::RakCodec;
use crate::protocol::packets::connected_ping::ConnectedPing;
use crate::protocol::packets::connection_request::ConnectionRequest;
use crate::protocol::packets::connection_request_accepted::ConnectionRequestAccepted;
use crate::protocol::packets::new_incoming_connection::NewIncomingConnection;
use crate::protocol::types::frame::Frame;
use crate::session::config::RakSessionConfig;
use crate::session::event::RakSessionEvent;
use crate::session::state::RakSessionState;
use crate::types::priority::RakPriority;
use crate::types::reliability::RakReliability;
use crate::util::constants::UDP_HEADER_SIZE;
use crate::util::socket_addr::get_overhead;
use std::io::Cursor;
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::debug;

pub struct RakSessionInner {
    event_tx: UnboundedSender<RakSessionEvent>,

    addr: SocketAddr,
    guid: u64,
    mtu: u16,
    config: RakSessionConfig,

    state: Mutex<RakSessionState>,

    last_update: Mutex<SystemTime>,
    last_ping: Mutex<SystemTime>,
    last_pong: Mutex<SystemTime>,

    in_tx: UnboundedSender<Vec<u8>>,
    out_tx: UnboundedSender<(Frame, RakPriority)>,
}

impl RakSessionInner {
    pub fn new<F>(event_tx: UnboundedSender<RakSessionEvent>, in_tx: UnboundedSender<Vec<u8>>, out_tx: UnboundedSender<(Frame, RakPriority)>, addr: SocketAddr, guid: u64, mtu: u16, conf: F) -> Self
    where
        F: FnOnce(&mut RakSessionConfig),
    {
        let mtu = mtu - UDP_HEADER_SIZE - get_overhead(&addr);
        let mut config = RakSessionConfig::default();
        conf(&mut config);

        Self {
            event_tx,
            addr,
            guid,
            mtu,
            config,

            state: Mutex::new(RakSessionState::Connecting),

            last_update: Mutex::new(SystemTime::now()),
            last_ping: Mutex::new(SystemTime::UNIX_EPOCH),
            last_pong: Mutex::new(SystemTime::UNIX_EPOCH),

            in_tx,
            out_tx,
        }
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn send(&self, buf: Vec<u8>, reliability: RakReliability, priority: RakPriority) {
        _ = self.out_tx.send((Frame::new(reliability, buf), priority));
    }

    pub fn inbound(&self, buf: Vec<u8>) {
        _ = self.in_tx.send(buf);
    }

    pub async fn handle_connection_request(&self, buf: &mut Cursor<&[u8]>) {
        let state = self.state.lock().await;
        match *state {
            RakSessionState::Connecting => (),
            _ => return debug!("unexpected ConnectionRequest from {}", self.addr),
        }

        let Ok(request) = ConnectionRequest::deserialize(buf) else {
            return debug!("failed to deserialize ConnectionRequest from {}", self.addr);
        };

        let accepted = ConnectionRequestAccepted {
            client_address: self.addr,
            system_index: 0,
            system_addresses: vec![],
            request_timestamp: request.client_timestamp,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
        };

        let mut buf = Vec::with_capacity(ConnectionRequestAccepted::size_hint(&accepted));
        ConnectionRequestAccepted::serialize(&accepted, &mut buf).unwrap();

        self.send(buf, RakReliability::ReliableOrdered, RakPriority::Normal);
    }

    pub async fn handle_new_incoming_connection(&self, buf: &mut Cursor<&[u8]>) {
        let mut state = self.state.lock().await;
        match *state {
            RakSessionState::Connecting => (),
            _ => return debug!("unexpected NewIncomingConnection from {}", self.addr),
        }

        let Ok(_) = NewIncomingConnection::deserialize(buf) else {
            return debug!("failed to deserialize NewIncomingConnection from {}", self.addr);
        };

        *state = RakSessionState::Connected;
        _ = self.event_tx.send(RakSessionEvent::Connected(self.addr));
    }

    pub async fn run_update_loop(self: Arc<Self>, mut in_rx: UnboundedReceiver<Vec<u8>>, mut out_rx: UnboundedReceiver<(Frame, RakPriority)>) {
        let mut tick_interval = tokio::time::interval(Duration::from_millis(10));
        let mut ping_interval = tokio::time::interval(Duration::from_millis(2000));

        loop {
            tokio::select! {
                Some((_, _)) = out_rx.recv() => {

                },
                Some(_) = in_rx.recv() => {

                },
                _ = tick_interval.tick() => {
                    // tick()
                },
                _ = ping_interval.tick() => {
                    let ping = ConnectedPing {
                        timestamp: UNIX_EPOCH.elapsed().unwrap().as_millis() as u64,
                    };

                    let mut buf = Vec::with_capacity(ConnectedPing::size_hint(&ping));
                    ConnectedPing::serialize(&ping, &mut buf).unwrap();
                },
            }
        }
    }
}
