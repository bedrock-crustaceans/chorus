use crate::network::BedrockProtocol;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::network::transport::SessionId;
use bedrock::network::codec::{decode_packets, encode_packets};
use bedrock::network::compression::Compression;
use bedrock::network::encryption::Encryption;
use bedrock::protocol::v662::enums::PlayStatus;
use bedrock::protocol::v662::packets::PlayStatusPacket;
use bedrock::protocol::v712::packets::{DisconnectMessage, DisconnectPacket};
use bedrock::protocol::v2193::enums::ConnectionFailReason;
use bevy_ecs::prelude::{Component, Entity, MessageWriter};
use std::collections::HashMap;
use std::mem::take;
use tracing::{debug, error};

pub mod state;

#[derive(Component)]
pub struct Session {
    entity: Entity,
    pub id: SessionId,

    closed: bool,
    state: SessionState,

    compression: Option<Compression>,
    encryption: Option<Encryption>,

    out_q: Vec<BedrockProtocol>,
    // already-encoded batches waiting to go out ahead of `out_q`, e.g. immediate sends
    // made under compression/encryption settings that have since changed
    pending_wire: Vec<Vec<u8>>,

    pub unhandled_packets: HashMap<&'static str, usize>,
}

impl Session {
    pub fn new(entity: Entity, id: SessionId) -> Self {
        Self {
            entity,
            id,

            closed: false,
            state: SessionState::Request,

            compression: None,
            encryption: None,

            out_q: vec![],
            pending_wire: vec![],

            unhandled_packets: HashMap::new(),
        }
    }

    /// Decodes a raw batch the transport handed us, dropping it if it fails to decrypt/decompress.
    pub fn decode(&mut self, stream: Vec<u8>) -> Vec<BedrockProtocol> {
        decode_packets(stream, self.compression.as_ref(), self.encryption.as_mut()).unwrap_or_else(|err| {
            error!("error decoding packets, dropping batch {:?}", err);
            vec![]
        })
    }

    /// Encodes the packet and sends it ahead of anything still queued in `out_q`, using the
    /// session's current compression/encryption so a state change right after this call doesn't
    /// retroactively apply to it.
    pub fn send_immediate(&mut self, packet: BedrockProtocol) {
        self.flush_queue();
        self.encode_now(vec![packet]);
    }

    pub fn send(&mut self, packet: BedrockProtocol) {
        self.out_q.push(packet);
    }

    fn flush_queue(&mut self) {
        let out = take(&mut self.out_q);
        if !out.is_empty() {
            self.encode_now(out);
        }
    }

    fn encode_now(&mut self, packets: Vec<BedrockProtocol>) {
        match encode_packets(&packets, self.compression.as_ref(), self.encryption.as_mut()) {
            Ok(stream) => self.pending_wire.push(stream),
            Err(err) => error!("error encoding packets, dropping batch {:?}", err),
        }
    }

    /// Drains everything encoded this tick for [`Network::flush`](crate::network::network::Network::flush)
    /// to hand to the transport, in the order it was produced.
    pub fn take_outgoing(&mut self) -> Vec<Vec<u8>> {
        self.flush_queue();
        take(&mut self.pending_wire)
    }

    pub fn set_compression(&mut self, compression: Option<Compression>) {
        debug!("Setting compression to {:?}", compression);
        self.compression = compression;
    }

    /// No-op for a NetherNet session: WebRTC's own DTLS already secures the connection, so this
    /// layer isn't needed and vanilla clients don't complete the handshake for it over NetherNet.
    pub fn set_encryption(&mut self, encryption: Option<Encryption>) {
        if !self.id.supports_encryption() {
            return;
        }

        debug!("Setting encryption");
        self.encryption = encryption;
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
