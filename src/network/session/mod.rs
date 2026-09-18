use crate::network::BedrockProtocol;
use crate::network::bandwidth::BandwidthCounters;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use bedrock::network::codec::{decode_packets, encode_packets};
use bedrock::network::compression::Compression;
use bedrock::network::connection::Connection;
use bedrock::network::encryption::Encryption;
use bedrock::network::error::ConnectionError;
use bedrock::protocol::Unknown;
use bedrock::protocol::v662::enums::PlayStatus;
use bedrock::protocol::v662::packets::PlayStatusPacket;
use bedrock::protocol::v712::packets::{DisconnectMessage, DisconnectPacket};
use bedrock::protocol::v1001::enums::ConnectionFailReason;
use bevy_ecs::prelude::{Component, Entity, Message, MessageWriter, Query};
use std::collections::HashMap;
use std::mem::take;
use std::sync::Arc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tracing::{debug, error};

pub mod state;

pub enum ConnectionEvent {
    Send(Vec<BedrockProtocol>),
    SetCompression(Option<Compression>),
    // box here otherwise it blows up the enum size (2080+ bytes)
    SetEncryption(Option<Box<Encryption>>),
}

enum ConnectionStep {
    Event(Option<ConnectionEvent>),
    Recv(Result<Vec<u8>, ConnectionError>),
}

#[derive(Message, Clone, Debug)]
pub struct PlayerKickedMessage {
    pub entity: Entity,
    pub reason: String,
}

#[derive(Message, Clone, Debug)]
pub struct PlayerDisconnectMessage {
    pub entity: Entity,
}

#[derive(Component)]
pub struct Session {
    entity: Entity,

    closed: bool,
    close_reason: Option<String>,
    close_notified: bool,
    state: SessionState,

    out_q: Vec<BedrockProtocol>,
    inc_rx: UnboundedReceiver<BedrockProtocol>,

    conn_tx: UnboundedSender<ConnectionEvent>,
    conn_task: JoinHandle<()>,

    pub unhandled_packets: HashMap<&'static str, usize>,
}

impl Session {
    pub fn new(entity: Entity, conn: Connection<Unknown>, runtime: &tokio::runtime::Runtime, bandwidth: Arc<BandwidthCounters>) -> Self {
        let (inc_tx, inc_rx) = tokio::sync::mpsc::unbounded_channel::<BedrockProtocol>();
        let (conn_tx, mut conn_rx) = tokio::sync::mpsc::unbounded_channel::<ConnectionEvent>();

        let mut conn: Connection<BedrockProtocol> = conn.into_ver();

        let conn_task = runtime.spawn(async move {
            'l: loop {
                // biased so that pending events are always applied before the next batch gets
                // decoded, otherwise a compression/encryption change could race an incoming batch
                let step = tokio::select! {
                    biased;
                    event = conn_rx.recv() => ConnectionStep::Event(event),
                    recv = conn.recv_raw() => ConnectionStep::Recv(recv),
                };

                match step {
                    ConnectionStep::Event(None) => break 'l,
                    ConnectionStep::Event(Some(ConnectionEvent::Send(packets))) => {
                        if packets.is_empty() {
                            continue;
                        }

                        // encoded here rather than in Connection::send so the bandwidth tracker
                        // gets to see how much actually goes over the wire
                        let stream = match encode_packets(&packets, conn.compression.as_ref(), conn.encryption.as_mut()) {
                            Ok(stream) => stream,
                            Err(err) => {
                                error!("error encoding packets, dropping batch {:?}", err);
                                continue;
                            }
                        };

                        bandwidth.add_sent(stream.len() as u64);

                        if let Err(err) = conn.send_raw(&stream).await {
                            error!("error sending packets to connection {:?}", err);
                            break 'l;
                        }
                    }
                    ConnectionStep::Event(Some(ConnectionEvent::SetCompression(compression))) => {
                        debug!("Setting compression to {:?}", compression);

                        conn.compression = compression;
                    }
                    ConnectionStep::Event(Some(ConnectionEvent::SetEncryption(encryption))) => {
                        debug!("Setting encryption");

                        conn.encryption = encryption.map(|b| *b);
                    }
                    ConnectionStep::Recv(Ok(stream)) => {
                        bandwidth.add_received(stream.len() as u64);

                        // a malformed batch is recoverable, so it only drops the batch
                        let packets = match decode_packets(stream, conn.compression.as_ref(), conn.encryption.as_mut()) {
                            Ok(packets) => packets,
                            Err(err) => {
                                error!("error decoding packets from connection, dropping batch {:?}", err);
                                continue;
                            }
                        };

                        for packet in packets {
                            if inc_tx.send(packet).is_err() {
                                break 'l;
                            }
                        }
                    }
                    ConnectionStep::Recv(Err(err)) => {
                        debug!("connection closed while receiving {:?}", err);
                        break 'l;
                    }
                }
            }
            conn.close().await;
        });

        Self {
            entity,

            closed: false,
            close_reason: None,
            close_notified: false,

            state: SessionState::Request,

            out_q: vec![],
            inc_rx,

            conn_tx,
            conn_task,

            unhandled_packets: HashMap::new(),
        }
    }

    /// Sends the packet without waiting for the end of the tick. Flushes the queue first so that
    /// packets stay in the order they were produced in.
    pub fn send_immediate(&mut self, packet: BedrockProtocol) {
        self.flush();
        _ = self.conn_tx.send(ConnectionEvent::Send(vec![packet]));
    }

    pub fn send(&mut self, packet: BedrockProtocol) {
        self.out_q.push(packet);
    }

    pub fn flush(&mut self) {
        let out = take(&mut self.out_q);
        if !out.is_empty() {
            _ = self.conn_tx.send(ConnectionEvent::Send(out));
        }
    }

    pub fn recv(&mut self) -> Option<BedrockProtocol> {
        match self.inc_rx.try_recv() {
            Ok(packet) => Some(packet),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.close(None);
                None
            }
        }
    }

    pub fn set_compression(&self, compression: Option<Compression>) {
        _ = self.conn_tx.send(ConnectionEvent::SetCompression(compression));
    }

    pub fn set_encryption(&self, encryption: Option<Encryption>) {
        _ = self.conn_tx.send(ConnectionEvent::SetEncryption(encryption.map(Box::new)));
    }

    pub fn set_state(&mut self, state: SessionState, writer: &mut MessageWriter<SessionStateChangedMessage>) {
        if state == self.state {
            return;
        }

        writer.write(SessionStateChangedMessage {
            entity: self.entity,
            from: self.state.clone(),
            to: state.clone(),
        });

        self.state = state;

        debug!("set session state to {:?}", self.state);
    }

    pub fn get_state(&self) -> SessionState {
        self.state.clone()
    }

    pub fn close(&mut self, reason: Option<&str>) {
        if self.is_closed() {
            return;
        }

        if let Some(reason) = reason {
            self.send_immediate(BedrockProtocol::DisconnectPacket(
                DisconnectPacket {
                    reason: ConnectionFailReason::Disconnected,
                    message: Some(DisconnectMessage {
                        kick_message: reason.to_string(),
                        filtered_message: reason.to_string(),
                    }),
                }
                .into(),
            ));

            self.close_reason = Some(reason.to_string());
        }

        self.closed = true;
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    fn take_close_event(&mut self) -> Option<Option<String>> {
        if !self.closed || self.close_notified {
            return None;
        }

        self.close_notified = true;
        Some(self.close_reason.take())
    }

    pub fn on_login_success(&mut self) {
        self.send_play_status(PlayStatus::LoginSuccess, false);
    }

    pub fn send_play_status(&mut self, status: PlayStatus, immediate: bool) {
        debug!("Sending play status: {:?}", status);

        let packet = BedrockProtocol::PlayStatusPacket(PlayStatusPacket { status }.into());
        if immediate {
            self.send_immediate(packet);
        } else {
            self.send(packet);
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.conn_task.abort();
    }
}

pub fn detect_session_close(mut sessions: Query<(Entity, &mut Session)>, mut kick_writer: MessageWriter<PlayerKickedMessage>, mut disconnect_writer: MessageWriter<PlayerDisconnectMessage>) {
    for (entity, mut session) in sessions.iter_mut() {
        match session.take_close_event() {
            Some(Some(reason)) => {
                kick_writer.write(PlayerKickedMessage { entity, reason });
            }
            Some(None) => {
                disconnect_writer.write(PlayerDisconnectMessage { entity });
            }
            None => {}
        }
    }
}
