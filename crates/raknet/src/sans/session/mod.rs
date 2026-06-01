pub mod congestion_controller;
pub mod event;
pub mod read;
pub mod write;

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
use crate::sans::session::congestion_controller::RakCongestionController;
use crate::sans::session::event::Eout;
use crate::sans::session::read::{Rin, Rout};
use crate::sans::session::write::{Win, Wout};
use crate::session::config::RakSessionConfig;
use crate::session::state::RakSessionState;
use crate::types::priority::RakPriority;
use crate::types::reliability::RakReliability;
use crate::util::constants::{DGRAM_HEADER_SIZE, DGRAM_MTU_OVERHEAD, UDP_HEADER_SIZE};
use crate::util::socket_addr::get_overhead;
use crate::util::{flags, packet_id};
use sansio::Protocol;
use std::cmp::{Reverse, min};
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::io::Cursor;
use std::mem::{replace, take};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::debug;

#[derive(Default, Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct RakSessionId(pub u64);

pub struct RakSession {
    pub id: RakSessionId,
    pub addr: SocketAddr,
    pub state: RakSessionState,
    guid: u64,
    mtu: u16,
    config: RakSessionConfig,

    last_tick: SystemTime,
    last_ping: SystemTime,
    last_recv: SystemTime,
    last_pong: SystemTime,

    congestion_controller: RakCongestionController,

    queue: VecDeque<(Vec<u8>, SocketAddr)>,

    sequences_recv: HashSet<u32>,
    sequences_lost: HashSet<u32>,

    outbound_seq: u32,
    outbound_spl: u16,
    outbound_rel: u32,
    outbound_queue: VecDeque<Frame>,
    outbound_cache: HashMap<u32, FrameSet>,
    outbound_resend: BinaryHeap<(Reverse<SystemTime>, u32)>,
    outbound_ord_idx: [u32; 32],
    outbound_seq_idx: [u32; 32],

    inbound_seq: u32,
    inbound_spl_queue: HashMap<u16, HashMap<u32, Frame>>,
    inbound_ord_queue: HashMap<u8, HashMap<u32, Frame>>,
    inbound_ord_idx: [u32; 32],
    inbound_seq_idx: [u32; 32],

    rout: VecDeque<Rout>,
    wout: VecDeque<Wout>,
    eout: VecDeque<Eout>,
}

impl Protocol<Rin, Win, ()> for RakSession {
    type Rout = Rout;
    type Wout = Wout;
    type Eout = Eout;
    type Error = ();
    type Time = SystemTime;

    fn handle_read(&mut self, msg: Rin) -> Result<(), Self::Error> {
        match msg {
            Rin::Datagram(buf, now) => {
                let Some(&b) = buf.first() else {
                    return Ok(());
                };

                let mut cursor = Cursor::new(buf.as_slice());
                match b {
                    _ if b & flags::VALID == 0 => debug!("received unknown online packet {:02X} from {}", b, self.addr),
                    _ if b & (flags::ACK | flags::NACK) != 0 => self.handle_ack(&mut cursor, now),
                    _ => self.handle_frame_set(&mut cursor, now),
                }
            }
        }
        Ok(())
    }

    fn poll_read(&mut self) -> Option<Self::Rout> {
        self.rout.pop_front()
    }

    fn handle_write(&mut self, msg: Win) -> Result<(), Self::Error> {
        match msg {
            Win::Frame(frame, priority, now) => self.send_frame(frame, priority, now),
        }
        Ok(())
    }

    fn poll_write(&mut self) -> Option<Self::Wout> {
        self.wout.pop_front()
    }

    fn handle_timeout(&mut self, now: Self::Time) -> Result<(), Self::Error> {
        if now >= self.last_recv + Duration::from_millis(15000) {
            debug!("detected stale connection from {}, disconnecting...", self.addr);

            self.disconnect_internal(true, true, now);
            return Ok(());
        }

        if now >= self.last_tick + Duration::from_millis(10) {
            self.tick(now);
        }

        if now >= self.last_ping + Duration::from_millis(2000) {
            let ping = ConnectedPing {
                timestamp: now.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
            };

            let mut buf = Vec::with_capacity(ConnectedPing::size_hint(&ping));
            ConnectedPing::serialize(&ping, &mut buf).unwrap();
        }

        Ok(())
    }

    fn poll_timeout(&mut self) -> Option<Self::Time> {
        [
            self.last_tick + Duration::from_millis(10),
            self.last_ping + Duration::from_millis(2000),
            self.last_recv + Duration::from_millis(15000),
        ]
        .into_iter()
        .min()
    }
}

impl RakSession {
    pub fn new<F>(id: RakSessionId, addr: SocketAddr, guid: u64, mtu: u16, conf: F) -> Self
    where
        F: FnOnce(&mut RakSessionConfig),
    {
        let mtu = mtu - UDP_HEADER_SIZE - get_overhead(&addr);
        let mut config = RakSessionConfig::default();
        conf(&mut config);

        let now = SystemTime::now();

        Self {
            id,
            addr,
            guid,
            mtu,
            config,

            last_tick: now,
            last_ping: now,
            last_recv: now,
            last_pong: now,

            state: RakSessionState::Connecting,
            congestion_controller: RakCongestionController::new(mtu as usize),

            sequences_recv: HashSet::new(),
            sequences_lost: HashSet::new(),

            queue: VecDeque::new(),

            outbound_seq: 0,
            outbound_spl: 0,
            outbound_rel: 0,
            outbound_queue: VecDeque::new(),
            outbound_cache: HashMap::new(),
            outbound_resend: BinaryHeap::new(),
            outbound_ord_idx: [0; 32],
            outbound_seq_idx: [0; 32],

            inbound_seq: 0,
            inbound_spl_queue: HashMap::new(),
            inbound_ord_queue: HashMap::new(),
            inbound_ord_idx: [0; 32],
            inbound_seq_idx: [0; 32],

            rout: VecDeque::new(),
            wout: VecDeque::new(),
            eout: VecDeque::new(),
        }
    }

    pub fn get_addr(&mut self) -> SocketAddr {
        self.addr
    }

    pub fn send(&mut self, buf: Vec<u8>, reliability: RakReliability, priority: RakPriority, now: SystemTime) {
        _ = self.handle_write(Win::Frame(Frame::new(reliability, buf), priority, now));
    }

    pub fn inbound(&mut self, buf: Vec<u8>, now: SystemTime) {
        _ = self.handle_read(Rin::Datagram(buf, now));
    }

    pub fn tick(&mut self, now: SystemTime) {
        match self.state {
            RakSessionState::Disconnecting | RakSessionState::Disconnected => return,
            _ => {}
        }

        if !self.sequences_recv.is_empty() {
            let ack = Ack {
                is_nack: false,
                sequences: self.sequences_recv.drain().collect(),
            };

            let mut buf = Vec::with_capacity(ack.size_hint());
            ack.serialize(&mut buf).unwrap();

            self.queue.push_back((buf, self.addr));
        }

        if !self.sequences_lost.is_empty() {
            let nack = Ack {
                is_nack: true,
                sequences: self.sequences_lost.drain().collect(),
            };

            let mut buf = Vec::with_capacity(nack.size_hint());
            nack.serialize(&mut buf).unwrap();

            self.queue.push_back((buf, self.addr));
        }

        self.send_stale(now);
        self.send_queue(now);
        self.flush();
    }

    fn send_stale(&mut self, now: SystemTime) {
        let mut pending = Vec::new();

        let mut bandwidth = { self.congestion_controller.retransmission_bandwidth() };

        while let Some(&(Reverse(sent), seq)) = self.outbound_resend.peek() {
            if sent > now {
                break;
            }

            let Some(set) = self.outbound_cache.get(&seq) else {
                self.outbound_resend.pop();
                continue;
            };

            let size = set.size_hint();
            if size > bandwidth {
                break;
            }
            bandwidth -= size;

            self.outbound_resend.pop();

            let set = self.outbound_cache.remove(&seq).expect("unreachable");
            pending.push(set);
        }

        for set in pending {
            self.send_frame_set(set, false, now);
        }
    }

    fn send_queue(&mut self, now: SystemTime) {
        let mut bandwidth = { self.congestion_controller.transmission_bandwidth() };

        let frames = {
            let mut frames = Vec::new();
            while let Some(frame) = self.outbound_queue.pop_front_if(|f| f.size_hint() <= bandwidth) {
                bandwidth -= frame.size_hint();
                frames.push(frame);
            }
            frames
        };

        if frames.is_empty() {
            return;
        };

        let sets = self.make_sets(frames);
        for set in sets {
            self.send_frame_set(set, false, now);
        }
    }

    fn make_sets(&mut self, frames: Vec<Frame>) -> Vec<FrameSet> {
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
                    sequence: self.outbound_seq,
                    frames: take(&mut batch),
                    continuous_send,
                    needs_b_and_as: true,
                    is_pair: false,
                });
                self.outbound_seq += 1;

                size = DGRAM_HEADER_SIZE as usize;
            }

            size += frame_size;
            batch.push(frame);
        }

        if !batch.is_empty() {
            let continuous_send = batch.iter().any(Frame::is_split);

            sets.push(FrameSet {
                sequence: self.outbound_seq,
                frames: batch,
                continuous_send,
                needs_b_and_as: true,
                is_pair: false,
            });
            self.outbound_seq += 1;
        }

        sets
    }

    fn send_frame_set(&mut self, frameset: FrameSet, immediate: bool, now: SystemTime) {
        let mut buf = Vec::with_capacity(frameset.size_hint());
        frameset.serialize(&mut buf).unwrap();

        match immediate {
            true => self.wout.push_back(Wout::Datagram(buf, self.addr)),
            false => {
                self.queue.push_back((buf, self.addr));
            }
        }

        let reliable = frameset.frames.iter().any(|f| f.reliability.is_reliable());
        if reliable {
            let resend = now + self.congestion_controller.retransmission_timeout();

            if !self.outbound_cache.contains_key(&frameset.sequence) {
                self.congestion_controller.sent(frameset.sequence, frameset.size_hint(), now);
            }
            self.outbound_resend.push((Reverse(resend), frameset.sequence));
            self.outbound_cache.insert(frameset.sequence, frameset);
        }
    }

    fn flush(&mut self) {
        for (buf, addr) in self.queue.drain(..) {
            self.wout.push_back(Wout::Datagram(buf, addr));
        }
    }

    fn send_frame(&mut self, frame: Frame, priority: RakPriority, now: SystemTime) {
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
            split_id = self.outbound_spl;
            self.outbound_spl += 1;

            let split_size = frame.payload.len().div_ceil(max_size);

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
            ord_idx = self.outbound_ord_idx[order_channel as usize];
            seq_idx = {
                let r = &mut self.outbound_seq_idx[order_channel as usize];
                let val = *r;
                *r += 1;
                val
            };
        } else if frame.reliability.is_ordered() {
            ord_idx = {
                let r = &mut self.outbound_ord_idx[order_channel as usize];
                let val = *r;
                *r += 1;
                val
            };
            self.outbound_seq_idx[order_channel as usize] = 0;
        }

        let split_size = payloads.len();
        let frames = payloads
            .into_iter()
            .enumerate()
            .map(|(i, payload)| Frame {
                reliability,
                payload,
                reliable_index: match reliability.is_reliable() {
                    true => {
                        let val = self.outbound_rel;
                        self.outbound_seq += 1;
                        val
                    }
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

        self.queue_frames(frames, priority, now);
    }

    fn queue_frames(&mut self, frames: Vec<Frame>, priority: RakPriority, now: SystemTime) {
        match priority {
            RakPriority::Immediate => {
                let sets = self.make_sets(frames);
                for set in sets {
                    self.send_frame_set(set, true, now);
                }
            }
            _ => {
                self.outbound_queue.extend(frames);
            }
        }
    }

    fn handle_ack(&mut self, buf: &mut Cursor<&[u8]>, now: SystemTime) {
        let Ok(ack) = Ack::deserialize(buf) else {
            return debug!("failed to deserialize Ack from {}", self.addr);
        };

        for seq in ack.sequences {
            let Some(set) = self.outbound_cache.remove(&seq) else {
                continue;
            };
            match ack.is_nack {
                true => {
                    self.queue_frames(set.frames, RakPriority::Immediate, now);
                    self.congestion_controller.nacked();
                }
                false => {
                    self.congestion_controller.acked(now, set.sequence, set.size_hint(), self.inbound_seq);
                }
            }
        }
    }

    fn handle_frame_set(&mut self, buf: &mut Cursor<&[u8]>, now: SystemTime) {
        let Ok(set) = FrameSet::deserialize(buf) else {
            return debug!("failed to deserialize FrameSet from {}", self.addr);
        };

        if self.sequences_recv.contains(&set.sequence) {
            debug!("received duplicate FrameSet {} from {}", set.sequence, self.addr);
        }
        self.sequences_recv.insert(set.sequence);

        self.sequences_lost.remove(&set.sequence);

        let inbound_seq = replace(&mut self.inbound_seq, set.sequence + 1);
        if set.sequence < inbound_seq {
            debug!("received out of order FrameSet {} from {}, expected {}", set.sequence, self.addr, inbound_seq);
        }

        if set.sequence > inbound_seq {
            self.sequences_lost.extend(inbound_seq..set.sequence);
        }

        for frame in set.frames {
            self.handle_frame(frame, now);
        }
    }

    fn handle_frame(&mut self, frame: Frame, now: SystemTime) {
        match frame.is_split() {
            true => self.handle_split_frame(frame, now),
            false => self.handle_full_frame(frame, now),
        }
    }

    fn handle_full_frame(&mut self, frame: Frame, now: SystemTime) {
        if frame.reliability.is_sequenced() {
            if frame.sequence_index < self.inbound_seq_idx[frame.order_channel as usize] || frame.order_index < self.inbound_ord_idx[frame.order_channel as usize] {
                debug!("received out of order FrameSet {} from {}", frame.order_channel, self.addr);
            }

            self.inbound_seq_idx[frame.order_channel as usize] = frame.sequence_index + 1;

            return self.handle_packet(frame.payload, now);
        }

        if frame.reliability.is_ordered() {
            if frame.order_index == self.inbound_ord_idx[frame.order_channel as usize] {
                self.inbound_seq_idx[frame.order_channel as usize] = 0;
                self.inbound_ord_idx[frame.order_channel as usize] = frame.order_index + 1;

                self.handle_packet(frame.payload, now);

                let mut idx = self.inbound_ord_idx[frame.order_channel as usize];

                let mut packets = Vec::new();
                {
                    let unord_queue = self.inbound_ord_queue.entry(frame.order_channel).or_default();
                    loop {
                        let Some(unord_frame) = unord_queue.remove(&idx) else {
                            break;
                        };

                        packets.push(unord_frame.payload);

                        idx += 1;
                    }
                }
                self.inbound_ord_idx[frame.order_channel as usize] = idx;

                for packet in packets {
                    self.handle_packet(packet, now);
                }
                return;
            }

            if frame.order_index > self.inbound_ord_idx[frame.order_channel as usize] {
                {
                    let unord_queue = self.inbound_ord_queue.entry(frame.order_channel).or_default();

                    unord_queue.insert(frame.order_index, frame);
                }
                return;
            }
            return;
        }

        self.handle_packet(frame.payload, now);
    }

    fn handle_split_frame(&mut self, frame: Frame, now: SystemTime) {
        let mut frame = frame;

        let fragments = self.inbound_spl_queue.entry(frame.split_id).or_default();
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

            self.inbound_spl_queue.remove(&frame.split_id);

            frame.payload = payload;
            frame.split_size = 0;
            frame.split_id = 0;
            frame.split_index = 0;

            self.handle_full_frame(frame, now);
        }
    }

    fn handle_packet(&mut self, buf: Vec<u8>, now: SystemTime) {
        let Some(&b) = buf.first() else {
            return;
        };

        let mut cursor = Cursor::new(buf.as_slice());
        match b {
            packet_id::CONNECTED_PING => self.handle_connected_ping(&mut cursor, now),
            packet_id::CONNECTED_PONG => self.handle_connected_pong(&mut cursor, now),
            packet_id::DISCONNECT => self.handle_disconnect(&mut cursor, now),
            _ => self.rout.push_back(Rout::Datagram(buf)),
        }
    }

    fn handle_connected_ping(&mut self, buf: &mut Cursor<&[u8]>, now: SystemTime) {
        let Ok(ping) = ConnectedPing::deserialize(buf) else {
            return debug!("failed to deserialize ConnectedPing from {}", self.addr);
        };

        let pong = ConnectedPong {
            ping_timestamp: ping.timestamp,
            timestamp: now.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
        };

        let mut buf = Vec::with_capacity(pong.size_hint());
        pong.serialize(&mut buf).unwrap();

        self.send(buf, RakReliability::Unreliable, RakPriority::Immediate, now);
    }

    fn handle_connected_pong(&mut self, buf: &mut Cursor<&[u8]>, now: SystemTime) {
        let Ok(pong) = ConnectedPong::deserialize(buf) else {
            return debug!("failed to deserialize ConnectedPong from {}", self.addr);
        };

        if UNIX_EPOCH + Duration::from_millis(pong.ping_timestamp) >= self.last_ping {
            self.last_pong = now;
        }
    }

    fn handle_disconnect(&mut self, buf: &mut Cursor<&[u8]>, now: SystemTime) {
        let Ok(_) = Disconnect::deserialize(buf) else {
            return debug!("failed to deserialize Disconnect from {}", self.addr);
        };

        debug!("session closed by {}", self.addr);

        self.disconnect_internal(false, true, now);
    }

    pub fn disconnect(&mut self, now: SystemTime) {
        let connected = self.state == RakSessionState::Connected;

        self.disconnect_internal(connected, connected, now);
    }

    fn disconnect_internal(&mut self, send: bool, connected: bool, now: SystemTime) {
        match self.state {
            RakSessionState::Disconnecting | RakSessionState::Disconnected => {
                return;
            }
            _ => {}
        }
        self.state = RakSessionState::Disconnecting;

        if send {
            let disconnect = Disconnect;

            let frame = Frame::new(RakReliability::ReliableOrdered, {
                let mut buf = Vec::with_capacity(disconnect.size_hint());
                disconnect.serialize(&mut buf).unwrap();
                buf
            });

            self.send_frame(frame, RakPriority::Immediate, now);
        }

        self.state = RakSessionState::Disconnected;

        if connected {
            self.eout.push_back(Eout::Disconnected(self.addr));
        }
    }
}
