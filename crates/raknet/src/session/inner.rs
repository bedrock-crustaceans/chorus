use crate::protocol::codec::RakCodec;
use crate::protocol::packets::ack::Ack;
use crate::protocol::packets::connected_ping::ConnectedPing;
use crate::protocol::packets::connection_request::ConnectionRequest;
use crate::protocol::packets::connection_request_accepted::ConnectionRequestAccepted;
use crate::protocol::packets::frame_set::FrameSet;
use crate::protocol::packets::new_incoming_connection::NewIncomingConnection;
use crate::protocol::types::frame::Frame;
use crate::session::config::RakSessionConfig;
use crate::session::congestion_controller::RakCongestionController;
use crate::session::event::RakSessionEvent;
use crate::session::state::RakSessionState;
use crate::types::priority::RakPriority;
use crate::types::reliability::RakReliability;
use crate::util::constants::{DGRAM_HEADER_SIZE, UDP_HEADER_SIZE};
use crate::util::socket_addr::get_overhead;
use std::any::Any;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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
    congestion_controller: Mutex<RakCongestionController>,

    last_update: Mutex<SystemTime>,
    last_ping: Mutex<SystemTime>,
    last_pong: Mutex<SystemTime>,

    in_tx: UnboundedSender<Vec<u8>>,
    out_tx: UnboundedSender<(Frame, RakPriority)>,

    queue: Mutex<VecDeque<(Vec<u8>, SocketAddr)>>,

    sequences_recv: Mutex<HashSet<u32>>,
    sequences_lost: Mutex<HashSet<u32>>,

    outbound_seq: Mutex<u32>,
    outbound_queue: Mutex<VecDeque<Frame>>,
    outbound_cache: Mutex<HashMap<u32, FrameSet>>,
    outbound_resend: Mutex<BinaryHeap<(Reverse<Instant>, u32)>>,
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
            congestion_controller: Mutex::new(RakCongestionController::new(mtu as usize)),

            last_update: Mutex::new(SystemTime::now()),
            last_ping: Mutex::new(SystemTime::UNIX_EPOCH),
            last_pong: Mutex::new(SystemTime::UNIX_EPOCH),

            in_tx,
            out_tx,

            sequences_recv: Mutex::new(HashSet::new()),
            sequences_lost: Mutex::new(HashSet::new()),

            queue: Mutex::new(VecDeque::new()),

            outbound_seq: Mutex::new(0),
            outbound_queue: Mutex::new(VecDeque::new()),
            outbound_cache: Mutex::new(HashMap::new()),
            outbound_resend: Mutex::new(BinaryHeap::new()),
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

    pub async fn tick(&self) {
        {
            let state = self.state.lock().await;
            match *state {
                RakSessionState::Disconnecting | RakSessionState::Disconnected => return,
                _ => {}
            }
        };

        {
            let mut sequences_recv = self.sequences_recv.lock().await;
            if !sequences_recv.is_empty() {
                let ack = Ack {
                    is_nack: false,
                    sequences: sequences_recv.drain().collect(),
                };

                let mut buf = Vec::with_capacity(ack.size_hint());
                ack.serialize(&mut buf).unwrap();

                let mut queue = self.queue.lock().await;
                queue.push_back((buf, self.addr));
            }
        };

        {
            let mut sequences_lost = self.sequences_lost.lock().await;
            if !sequences_lost.is_empty() {
                let nack = Ack {
                    is_nack: true,
                    sequences: sequences_lost.drain().collect(),
                };

                let mut buf = Vec::with_capacity(nack.size_hint());
                nack.serialize(&mut buf).unwrap();

                let mut queue = self.queue.lock().await;
                queue.push_back((buf, self.addr));
            }
        };

        self.send_stale().await;
        self.send_queue().await;
        self.flush().await;
    }

    async fn send_stale(&self) {
        let now = Instant::now();

        let mut pending = Vec::new();

        let mut bandwidth = {
            let congestion_controller = self.congestion_controller.lock().await;
            congestion_controller.retransmission_bandwidth()
        };

        {
            let mut outbound_resend = self.outbound_resend.lock().await;
            let mut outbound_cache = self.outbound_cache.lock().await;
            while let Some(&(Reverse(sent), seq)) = outbound_resend.peek() {
                if sent > now {
                    break;
                }

                let Some(set) = outbound_cache.get(&seq) else {
                    outbound_resend.pop();
                    continue;
                };

                let size = set.size_hint();
                if size > bandwidth {
                    break;
                }
                bandwidth -= size;

                outbound_resend.pop();

                let set = outbound_cache.remove(&seq).expect("unreachable");
                pending.push(set);
            }
        };

        for set in pending {
            self.send_frame_set(set, false).await;
        }
    }

    async fn send_queue(&self) {
        let mut bandwidth = {
            let congestion_controller = self.congestion_controller.lock().await;
            congestion_controller.transmission_bandwidth()
        };

        let frames = {
            let mut frames = Vec::new();
            let mut outbound_queue = self.outbound_queue.lock().await;
            while let Some(frame) = outbound_queue.pop_front_if(|f| f.size_hint() > bandwidth) {
                bandwidth -= frame.size_hint();
                frames.push(frame);
            }
            frames
        };

        let sets = self.make_sets(frames).await;
        for set in sets {
            self.send_frame_set(set, false).await;
        }
    }

    async fn make_sets(&self, frames: Vec<Frame>) -> Vec<FrameSet> {
        let mut sets = Vec::new();

        let max = (self.mtu - DGRAM_HEADER_SIZE) as usize;

        let mut batch = Vec::new();
        let mut size = DGRAM_HEADER_SIZE as usize;

        let mut outbound_seq = self.outbound_seq.lock().await;

        for frame in frames {
            let frame_size = frame.size_hint();

            if frame_size > max {
                panic!("Frame too large for FrameSet, size: {}, max size: {}", frame_size, max);
            }

            if size + frame_size > max {
                let continuous_send = batch.iter().any(Frame::is_split);

                sets.push(FrameSet {
                    sequence: *outbound_seq,
                    frames: batch.clone(),
                    continuous_send,
                    needs_b_and_as: true,
                    is_pair: false,
                });

                *outbound_seq += 1;
                batch.clear();
                size = DGRAM_HEADER_SIZE as usize;
            }

            size += frame_size;
            batch.push(frame.clone());
        }

        if !batch.is_empty() {
            let continuous_send = batch.iter().any(Frame::is_split);

            sets.push(FrameSet {
                sequence: *outbound_seq,
                frames: batch.clone(),
                continuous_send,
                needs_b_and_as: true,
                is_pair: false,
            });

            *outbound_seq += 1;
        }

        sets
    }

    async fn send_frame_set(&self, frameset: FrameSet, immediate: bool) {
        let mut buf = Vec::with_capacity(frameset.size_hint());
        frameset.serialize(&mut buf).unwrap();

        match immediate {
            true => self.event_tx.send(RakSessionEvent::Outbound(buf, self.addr)).unwrap(),
            false => {
                let mut queue = self.queue.lock().await;
                queue.push_back((buf, self.addr));
            }
        }

        let reliable = frameset.frames.iter().any(|f| f.reliability.is_reliable());
        if reliable {
            let mut outbound_cache = self.outbound_cache.lock().await;
            let mut outbound_resend = self.outbound_resend.lock().await;
            let mut congestion_controller = self.congestion_controller.lock().await;

            let resend = Instant::now() + congestion_controller.retransmission_timeout();

            if !outbound_cache.contains_key(&frameset.sequence) {
                congestion_controller.sent(frameset.sequence, frameset.size_hint());
            }
            outbound_resend.push((Reverse(resend), frameset.sequence));
            outbound_cache.insert(frameset.sequence, frameset);
        }
    }

    async fn flush(&self) {
        let mut queue = self.queue.lock().await;
        for (buf, addr) in queue.drain(..) {
            self.event_tx.send(RakSessionEvent::Outbound(buf, addr)).unwrap();
        }
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
                    self.tick().await;
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
