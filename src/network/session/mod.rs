use crate::network::BedrockProtocol;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use bedrock::network::compression::Compression;
use bedrock::network::connection::Connection;
use bedrock::network::encryption::Encryption;
use bedrock::protocol::Unknown;
use bedrock::protocol::v662::enums::PlayStatus;
use bedrock::protocol::v662::packets::PlayStatusPacket;
use bedrock::protocol::v712::packets::{DisconnectMessage, DisconnectPacket};
use bedrock::protocol::v1001::enums::ConnectionFailReason;
use bevy_ecs::prelude::{Component, Entity, MessageWriter};
use bevy_tasks::futures::now_or_never;
use std::collections::HashMap;
use std::mem::take;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tracing::{debug, error};

pub mod state;

pub enum ConnectionEvent {
    Recv,
    Send(Vec<BedrockProtocol>),
    SetCompression(Option<Compression>),
    // box here otherwise it blows up the enum size (2080+ bytes)
    SetEncryption(Option<Box<Encryption>>),
}

#[derive(Component)]
pub struct Session {
    entity: Entity,

    closed: bool,
    state: SessionState,

    out_q: Vec<BedrockProtocol>,
    inc_rx: UnboundedReceiver<BedrockProtocol>,

    conn_tx: UnboundedSender<ConnectionEvent>,
    conn_task: JoinHandle<()>,

    pub unhandled_packets: HashMap<&'static str, usize>,
}

impl Session {
    pub fn new(entity: Entity, conn: Connection<Unknown>, runtime: &tokio::runtime::Runtime) -> Self {
        let (inc_tx, inc_rx) = tokio::sync::mpsc::unbounded_channel::<BedrockProtocol>();
        let (conn_tx, mut conn_rx) = tokio::sync::mpsc::unbounded_channel::<ConnectionEvent>();

        let mut conn: Connection<BedrockProtocol> = conn.into_ver();

        let conn_task = runtime.spawn(async move {
            'l: loop {
                if conn.is_closed().await {
                    break 'l;
                }

                while let Ok(event) = conn_rx.try_recv() {
                    match event {
                        ConnectionEvent::Recv => {
                            if let Some(recv) = now_or_never(conn.recv()) {
                                match recv {
                                    Ok(packets) => {
                                        for packet in packets {
                                            if inc_tx.send(packet).is_err() {
                                                break 'l;
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        error!("error receiving packets from connection, dropping batch {:?}", err);
                                    }
                                }
                            }
                        }
                        ConnectionEvent::Send(packets) => {
                            if !packets.is_empty()
                                && let Err(err) = conn.send(&packets).await
                            {
                                error!("error sending packets to connection {:?}", err);
                                break 'l;
                            }
                        }
                        ConnectionEvent::SetCompression(compression) => {
                            debug!("Setting compression to {:?}", compression);

                            conn.compression = compression;
                        }
                        ConnectionEvent::SetEncryption(encryption) => {
                            debug!("Setting encryption");

                            conn.encryption = encryption.map(|b| *b);
                        }
                    }
                }
            }
            conn.close().await;
        });

        Self {
            entity,

            closed: false,

            state: SessionState::Request,

            out_q: vec![],
            inc_rx,

            conn_tx,
            conn_task,

            unhandled_packets: HashMap::new(),
        }
    }

    pub fn send_immediate(&self, packet: BedrockProtocol) {
        _ = self.conn_tx.send(ConnectionEvent::Send(vec![packet]));
    }

    pub fn send(&mut self, packet: BedrockProtocol) {
        self.out_q.push(packet);
    }

    pub fn tick(&mut self) {
        let out = take(&mut self.out_q);
        if !out.is_empty() {
            _ = self.conn_tx.send(ConnectionEvent::Send(out));
        }
        _ = self.conn_tx.send(ConnectionEvent::Recv);
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
        }

        self.closed = true;
    }

    pub fn is_closed(&self) -> bool {
        self.closed
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
