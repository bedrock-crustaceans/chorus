use crate::protocol::codec::RakCodec;
use crate::protocol::packets::ack::Ack;
use crate::protocol::packets::connected_ping::ConnectedPing;
use crate::protocol::packets::connected_pong::ConnectedPong;
use crate::protocol::packets::connection_request::ConnectionRequest;
use crate::protocol::packets::connection_request_accepted::ConnectionRequestAccepted;
use crate::protocol::packets::disconnect::Disconnect;
use crate::protocol::packets::frame_set::FrameSet;
use crate::protocol::packets::new_incoming_connection::NewIncomingConnection;
use crate::protocol::types::frame::Frame;
use crate::session::config::RakSessionConfig;
use crate::session::congestion_controller::RakCongestionController;
use crate::session::event::RakSessionEvent;
use crate::session::state::RakSessionState;
use crate::types::priority::RakPriority;
use crate::types::reliability::RakReliability;
use crate::util::constants::{DGRAM_HEADER_SIZE, DGRAM_MTU_OVERHEAD, UDP_HEADER_SIZE};
use crate::util::socket_addr::get_overhead;
use crate::util::{flags, packet_id};
use std::array::from_fn;
use std::cmp::{Reverse, min};
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, AtomicU32, AtomicU64, Ordering};
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

    last_update: Mutex<Instant>,
    last_ping: AtomicU64,
    last_pong: AtomicU64,

    in_tx: UnboundedSender<Vec<u8>>,
    out_tx: UnboundedSender<(Frame, RakPriority)>,

    queue: Mutex<VecDeque<(Vec<u8>, SocketAddr)>>,

    sequences_recv: Mutex<HashSet<u32>>,
    sequences_lost: Mutex<HashSet<u32>>,

    outbound_seq: AtomicU32,
    outbound_spl: AtomicU16,
    outbound_rel: AtomicU32,
    outbound_queue: Mutex<VecDeque<Frame>>,
    outbound_cache: Mutex<HashMap<u32, FrameSet>>,
    outbound_resend: Mutex<BinaryHeap<(Reverse<Instant>, u32)>>,
    outbound_ord_idx: [AtomicU32; 32],
    outbound_seq_idx: [AtomicU32; 32],

    inbound_seq: AtomicU32,
    inbound_spl_queue: Mutex<HashMap<u16, HashMap<u32, Frame>>>,
    inbound_ord_queue: Mutex<HashMap<u8, HashMap<u32, Frame>>>,
    inbound_ord_idx: [AtomicU32; 32],
    inbound_seq_idx: [AtomicU32; 32],
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

            last_update: Mutex::new(Instant::now()),
            last_ping: AtomicU64::new(0),
            last_pong: AtomicU64::new(0),

            in_tx,
            out_tx,

            sequences_recv: Mutex::new(HashSet::new()),
            sequences_lost: Mutex::new(HashSet::new()),

            queue: Mutex::new(VecDeque::new()),

            outbound_seq: AtomicU32::new(0),
            outbound_spl: AtomicU16::new(0),
            outbound_rel: AtomicU32::new(0),
            outbound_queue: Mutex::new(VecDeque::new()),
            outbound_cache: Mutex::new(HashMap::new()),
            outbound_resend: Mutex::new(BinaryHeap::new()),
            outbound_ord_idx: from_fn(|_| AtomicU32::new(0)),
            outbound_seq_idx: from_fn(|_| AtomicU32::new(0)),

            inbound_seq: AtomicU32::new(0),
            inbound_spl_queue: Mutex::new(HashMap::new()),
            inbound_ord_queue: Mutex::new(HashMap::new()),
            inbound_ord_idx: from_fn(|_| AtomicU32::new(0)),
            inbound_seq_idx: from_fn(|_| AtomicU32::new(0)),
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
            let last_update = self.last_update.lock().await;
            if last_update.elapsed() >= Duration::from_millis(15000) {
                debug!("detected stale connection from {}, disconnecting...", self.addr);

                return self.disconnect_internal(true, true).await;
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
            while let Some(frame) = outbound_queue.pop_front_if(|f| f.size_hint() <= bandwidth) {
                bandwidth -= frame.size_hint();
                frames.push(frame);
            }
            frames
        };

        if frames.is_empty() {
            return;
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

        for frame in frames {
            let frame_size = frame.size_hint();

            if frame_size > max {
                panic!("Frame too large for FrameSet, size: {}, max size: {}", frame_size, max);
            }

            if size + frame_size > max {
                let continuous_send = batch.iter().any(Frame::is_split);

                sets.push(FrameSet {
                    sequence: self.outbound_seq.fetch_add(1, Ordering::Relaxed),
                    frames: batch.clone(),
                    continuous_send,
                    needs_b_and_as: true,
                    is_pair: false,
                });

                batch.clear();
                size = DGRAM_HEADER_SIZE as usize;
            }

            size += frame_size;
            batch.push(frame.clone());
        }

        if !batch.is_empty() {
            let continuous_send = batch.iter().any(Frame::is_split);

            sets.push(FrameSet {
                sequence: self.outbound_seq.fetch_add(1, Ordering::Relaxed),
                frames: batch.clone(),
                continuous_send,
                needs_b_and_as: true,
                is_pair: false,
            });
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

    async fn send_frame(&self, frame: Frame, priority: RakPriority) {
        let max_size = (self.mtu - DGRAM_MTU_OVERHEAD) as usize;

        let order_channel = frame.order_channel;

        let mut reliability = frame.reliability;
        let mut split_id = 0;

        let payloads = if frame.size_hint() > max_size {
            reliability = match reliability {
                RakReliability::Unreliable => RakReliability::Reliable,
                RakReliability::UnreliableSequenced => RakReliability::ReliableSequenced,
                RakReliability::UnreliableWithAckReceipt => RakReliability::ReliableWithAckReceipt,
                val => val,
            };
            split_id = self.outbound_spl.fetch_add(1, Ordering::Relaxed);

            let split_size = (frame.payload.len() + max_size - 1) / max_size;

            let mut payloads = Vec::with_capacity(split_size);
            for i in 0..split_size {
                let start = i * max_size;
                let end = min(start + max_size, frame.payload.len());

                payloads.push(frame.payload[start..end].to_vec());
            }
            payloads
        } else {
            vec![frame.payload]
        };

        let mut ord_idx = 0;
        let mut seq_idx = 0;
        if frame.reliability.is_sequenced() {
            ord_idx = self.outbound_ord_idx[order_channel as usize].load(Ordering::Relaxed);
            seq_idx = self.outbound_seq_idx[order_channel as usize].fetch_add(1, Ordering::Relaxed);
        } else if frame.reliability.is_ordered() {
            ord_idx = self.outbound_ord_idx[order_channel as usize].fetch_add(1, Ordering::Relaxed);
            self.outbound_seq_idx[order_channel as usize].store(0, Ordering::Relaxed);
        }

        let split_size = payloads.len();
        let frames = payloads
            .into_iter()
            .enumerate()
            .map(|(i, payload)| Frame {
                reliability,
                payload,
                reliable_index: match reliability.is_reliable() {
                    true => self.outbound_rel.fetch_add(1, Ordering::Relaxed),
                    false => 0,
                },
                sequence_index: seq_idx,
                order_index: ord_idx,
                order_channel,
                split_size: if split_size > 1 { split_size as u32 } else { 0 },
                split_id,
                split_index: i as u32,
            })
            .collect();

        self.queue_frames(frames, priority).await;
    }

    async fn queue_frames(&self, frames: Vec<Frame>, priority: RakPriority) {
        match priority {
            RakPriority::Immediate => {
                let sets = self.make_sets(frames).await;
                for set in sets {
                    self.send_frame_set(set, true).await;
                }
            }
            _ => {
                let mut outbound_queue = self.outbound_queue.lock().await;
                outbound_queue.extend(frames);
            }
        }
    }

    async fn handle_ack(&self, buf: &mut Cursor<&[u8]>) {
        let Ok(ack) = Ack::deserialize(buf) else {
            return debug!("failed to deserialize Ack from {}", self.addr);
        };

        let now = Instant::now();
        for seq in ack.sequences {
            let set = {
                let mut outbound_cache = self.outbound_cache.lock().await;
                outbound_cache.remove(&seq)
            };
            let Some(set) = set else {
                continue;
            };
            match ack.is_nack {
                true => {
                    self.queue_frames(set.frames, RakPriority::Immediate).await;
                    self.congestion_controller.lock().await.nacked();
                }
                false => {
                    self.congestion_controller
                        .lock()
                        .await
                        .acked(now, set.sequence, set.size_hint(), self.inbound_seq.load(Ordering::Relaxed));
                }
            }
        }
    }

    async fn handle_frame_set(&self, buf: &mut Cursor<&[u8]>) {
        let Ok(set) = FrameSet::deserialize(buf) else {
            return debug!("failed to deserialize FrameSet from {}", self.addr);
        };

        {
            let mut sequences_recv = self.sequences_recv.lock().await;
            if sequences_recv.contains(&set.sequence) {
                debug!("received duplicate FrameSet {} from {}", set.sequence, self.addr);
            }
            sequences_recv.insert(set.sequence);
        };

        {
            let mut sequences_lost = self.sequences_lost.lock().await;
            sequences_lost.remove(&set.sequence);

            let inbound_seq = self.inbound_seq.swap(set.sequence + 1, Ordering::Relaxed);
            if set.sequence < inbound_seq {
                debug!("received out of order FrameSet {} from {}, expected {}", set.sequence, self.addr, inbound_seq);
            }

            if set.sequence > inbound_seq {
                sequences_lost.extend(inbound_seq..set.sequence);
            }
        };

        for frame in set.frames {
            self.handle_frame(frame).await;
        }
    }

    async fn handle_frame(&self, frame: Frame) {
        match frame.is_split() {
            true => self.handle_split_frame(frame).await,
            false => self.handle_full_frame(frame).await,
        }
    }

    async fn handle_full_frame(&self, frame: Frame) {
        if frame.reliability.is_sequenced() {
            if frame.sequence_index < self.inbound_seq_idx[frame.order_channel as usize].load(Ordering::Relaxed)
                || frame.order_index < self.inbound_ord_idx[frame.order_channel as usize].load(Ordering::Relaxed)
            {
                debug!("received out of order FrameSet {} from {}", frame.order_channel, self.addr);
            }

            self.inbound_seq_idx[frame.order_channel as usize].store(frame.sequence_index + 1, Ordering::Relaxed);

            return self.handle_packet(frame.payload).await;
        }

        if frame.reliability.is_ordered() {
            if frame.order_index == self.inbound_ord_idx[frame.order_channel as usize].load(Ordering::Relaxed) {
                self.inbound_seq_idx[frame.order_channel as usize].store(0, Ordering::Relaxed);
                self.inbound_ord_idx[frame.order_channel as usize].store(frame.order_index + 1, Ordering::Relaxed);

                self.handle_packet(frame.payload).await;

                let mut idx = self.inbound_ord_idx[frame.order_channel as usize].load(Ordering::Relaxed);

                let mut packets = Vec::new();
                {
                    let mut inbound_ord_queue = self.inbound_ord_queue.lock().await;
                    let unord_queue = inbound_ord_queue.entry(frame.order_channel).or_default();

                    loop {
                        let Some(unord_frame) = unord_queue.remove(&idx) else {
                            break;
                        };

                        packets.push(unord_frame.payload);

                        idx += 1;
                    }
                }
                self.inbound_ord_idx[frame.order_channel as usize].store(idx, Ordering::Relaxed);

                for packet in packets {
                    self.handle_packet(packet).await;
                }
                return;
            }

            if frame.order_index > self.inbound_ord_idx[frame.order_channel as usize].load(Ordering::Relaxed) {
                {
                    let mut inbound_ord_queue = self.inbound_ord_queue.lock().await;
                    let unord_queue = inbound_ord_queue.entry(frame.order_channel).or_default();

                    unord_queue.insert(frame.order_index, frame);
                }
                return;
            }
            return;
        }

        self.handle_packet(frame.payload).await;
    }

    async fn handle_split_frame(&self, frame: Frame) {
        let mut frame = frame;

        let mut inbound_spl_queue = self.inbound_spl_queue.lock().await;

        let fragments = inbound_spl_queue.entry(frame.split_id).or_default();
        fragments.insert(frame.split_index, frame.clone());

        if fragments.len() as u32 == frame.split_size {
            let mut payload = Vec::new();

            for i in 0..frame.split_size {
                let frag = match fragments.get(&i) {
                    Some(f) => f,
                    None => return,
                };
                payload.extend_from_slice(&frag.payload);
            }

            inbound_spl_queue.remove(&frame.split_id);

            frame.payload = payload;
            frame.split_size = 0;
            frame.split_id = 0;
            frame.split_index = 0;

            drop(inbound_spl_queue);

            self.handle_full_frame(frame).await;
        }
    }

    async fn handle_packet(&self, buf: Vec<u8>) {
        let Some(&b) = buf.first() else {
            return;
        };

        let mut cursor = Cursor::new(buf.as_slice());
        match b {
            packet_id::CONNECTED_PING => self.handle_connected_ping(&mut cursor).await,
            packet_id::CONNECTED_PONG => self.handle_connected_pong(&mut cursor).await,
            packet_id::DISCONNECT => self.handle_disconnect(&mut cursor).await,
            _ => self.event_tx.send(RakSessionEvent::Inbound(buf, self.addr)).unwrap(),
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

        debug!("handling connection request from {}", self.addr);

        let accepted = ConnectionRequestAccepted {
            client_address: self.addr,
            system_index: 0,
            system_addresses: vec![],
            request_timestamp: request.client_timestamp,
            timestamp: UNIX_EPOCH.elapsed().unwrap().as_millis() as u64,
        };

        let mut buf = Vec::with_capacity(ConnectionRequestAccepted::size_hint(&accepted));
        ConnectionRequestAccepted::serialize(&accepted, &mut buf).unwrap();

        self.send(buf, RakReliability::ReliableOrdered, RakPriority::Normal);
    }

    pub async fn handle_new_incoming_connection(&self, buf: &mut Cursor<&[u8]>) {
        let mut state = self.state.lock().await;
        match *state {
            RakSessionState::Connecting => {}
            _ => return debug!("unexpected NewIncomingConnection from {}", self.addr),
        }

        let Ok(_) = NewIncomingConnection::deserialize(buf) else {
            return debug!("failed to deserialize NewIncomingConnection from {}", self.addr);
        };

        debug!("handling new incoming connection from {}", self.addr);

        *state = RakSessionState::Connected;
        _ = self.event_tx.send(RakSessionEvent::Connected(self.addr));
    }

    async fn handle_connected_ping(&self, buf: &mut Cursor<&[u8]>) {
        let Ok(ping) = ConnectedPing::deserialize(buf) else {
            return debug!("failed to deserialize ConnectedPing from {}", self.addr);
        };

        let pong = ConnectedPong {
            ping_timestamp: ping.timestamp,
            timestamp: UNIX_EPOCH.elapsed().unwrap().as_millis() as u64,
        };

        let mut buf = Vec::with_capacity(pong.size_hint());
        pong.serialize(&mut buf).unwrap();

        self.send(buf, RakReliability::Unreliable, RakPriority::Immediate);
    }

    async fn handle_connected_pong(&self, buf: &mut Cursor<&[u8]>) {
        let Ok(pong) = ConnectedPong::deserialize(buf) else {
            return debug!("failed to deserialize ConnectedPong from {}", self.addr);
        };

        if pong.ping_timestamp >= self.last_ping.load(Ordering::Relaxed) {
            self.last_pong.store(UNIX_EPOCH.elapsed().unwrap().as_millis() as u64, Ordering::Relaxed);
        }
    }

    async fn handle_disconnect(&self, buf: &mut Cursor<&[u8]>) {
        let Ok(_) = Disconnect::deserialize(buf) else {
            return debug!("failed to deserialize Disconnect from {}", self.addr);
        };

        debug!("session closed by {}", self.addr);

        self.disconnect_internal(false, true).await;
    }

    pub async fn disconnect(&self) {
        let connected = { *self.state.lock().await == RakSessionState::Connected };

        self.disconnect_internal(connected, connected).await;
    }

    async fn disconnect_internal(&self, send: bool, connected: bool) {
        let mut state = self.state.lock().await;
        match *state {
            RakSessionState::Disconnecting | RakSessionState::Disconnected => {
                return;
            }
            _ => {}
        }
        *state = RakSessionState::Disconnecting;

        if send {
            let disconnect = Disconnect {};

            let frame = Frame {
                reliability: RakReliability::ReliableOrdered,
                payload: {
                    let mut buf = Vec::with_capacity(disconnect.size_hint());
                    disconnect.serialize(&mut buf).unwrap();
                    buf
                },
                reliable_index: 0,
                sequence_index: 0,
                order_index: 0,
                order_channel: 0,
                split_size: 0,
                split_id: 0,
                split_index: 0,
            };

            self.send_frame(frame, RakPriority::Immediate).await;
        }

        *state = RakSessionState::Disconnected;

        if connected {
            self.event_tx.send(RakSessionEvent::Disconnected(self.addr)).unwrap();
        }
    }

    pub async fn run_update_loop(self: Arc<Self>, mut in_rx: UnboundedReceiver<Vec<u8>>, mut out_rx: UnboundedReceiver<(Frame, RakPriority)>) {
        let mut tick_interval = tokio::time::interval(Duration::from_millis(10));
        let mut ping_interval = tokio::time::interval(Duration::from_millis(2000));

        loop {
            tokio::select! {
                Some(out) = out_rx.recv() => {
                    self.send_frame(out.0, out.1).await;
                },
                Some(buf) = in_rx.recv() => {
                    let Some(&b) = buf.first() else { continue; };

                    {
                        *self.last_update.lock().await = Instant::now();
                    };

                    let mut cursor = Cursor::new(buf.as_slice());
                    match b {
                        _ if b & flags::VALID == 0 => debug!("received unknown online packet {:02X} from {}", b, self.addr),
                        _ if b & (flags::ACK | flags::NACK) != 0 => self.handle_ack(&mut cursor).await,
                        _ => self.handle_frame_set(&mut cursor).await,
                    }
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
