use crate::network::BedrockProtocol;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::network::transport::SessionId;
use bedrock::auth::Login;
use bedrock::network::codec::{decode_packets, encode_packets};
use bedrock::network::compression::Compression;
use bedrock::network::encryption::Encryption;
use bedrock::network::error::LoginError;
use bedrock::network::login::LoginAction;
use bedrock::network::login::server::{LoginFailure, ServerLogin, ServerLoginAction, ServerLoginOptions};
use bedrock::protocol::v662::enums::PlayStatus;
use bedrock::protocol::v662::packets::PlayStatusPacket;
use bedrock::protocol::v712::packets::{DisconnectMessage, DisconnectPacket};
use bedrock::protocol::v2193::enums::ConnectionFailReason;
use bevy_ecs::prelude::{Component, Entity, MessageWriter};
use std::collections::HashMap;
use std::mem::take;
use tracing::{debug, error, warn};

pub mod state;

#[derive(Default)]
pub struct LoginProgress {
    pub settings_accepted: bool,
    pub completed: Option<Box<Login>>,
}

pub struct Batch {
    pub data: Vec<u8>,
    pub immediate: bool,
}

#[derive(Component)]
pub struct Session {
    entity: Entity,
    pub id: SessionId,

    closed: bool,
    state: SessionState,
    login: Option<ServerLogin>,

    compression: Option<Compression>,
    encryption: Option<Encryption>,

    out_q: Vec<BedrockProtocol>,
    // already-encoded batches waiting to go out ahead of `out_q`, e.g. immediate sends
    // made under compression/encryption settings that have since changed
    pending_wire: Vec<Batch>,

    pub unhandled_packets: HashMap<&'static str, usize>,
}

impl Session {
    pub fn new(entity: Entity, id: SessionId, login_options: ServerLoginOptions) -> Self {
        Self {
            entity,
            id,

            closed: false,
            state: SessionState::Negotiating,
            login: Some(ServerLogin::new(login_options)),

            compression: None,
            encryption: None,

            out_q: vec![],
            pending_wire: vec![],

            unhandled_packets: HashMap::new(),
        }
    }

    /// Decodes a raw batch the transport handed us, dropping it if it fails to decrypt/decompress.
    pub fn decode(&mut self, stream: Vec<u8>, max_batch_size: usize) -> Vec<BedrockProtocol> {
        decode_packets(stream, self.compression.as_ref(), self.encryption.as_mut(), max_batch_size).unwrap_or_else(|err| {
            error!("error decoding packets, dropping batch {:?}", err);
            vec![]
        })
    }

    /// Encodes the packet and sends it ahead of anything still queued in `out_q`, using the
    /// session's current compression/encryption so a state change right after this call doesn't
    /// retroactively apply to it.
    pub fn send_immediate(&mut self, packet: BedrockProtocol) {
        self.flush_queue();
        self.encode_now(vec![packet], true);
    }

    pub fn send(&mut self, packet: BedrockProtocol) {
        self.out_q.push(packet);
    }

    fn flush_queue(&mut self) {
        let out = take(&mut self.out_q);
        if !out.is_empty() {
            self.encode_now(out, false);
        }
    }

    fn encode_now(&mut self, packets: Vec<BedrockProtocol>, immediate: bool) {
        match encode_packets(&packets, self.compression.as_ref(), self.encryption.as_mut()) {
            Ok(data) => self.pending_wire.push(Batch { data, immediate }),
            Err(err) => error!("error encoding packets, dropping batch {:?}", err),
        }
    }

    /// Drains everything encoded this tick for [`Network::flush`](crate::network::network::Network::flush)
    /// to hand to the transport, in the order it was produced.
    pub fn take_outgoing(&mut self) -> Vec<Batch> {
        self.flush_queue();
        take(&mut self.pending_wire)
    }

    pub fn advance_login(&mut self, packet: BedrockProtocol) -> Result<LoginProgress, LoginFailure<BedrockProtocol>> {
        let Some(login) = self.login.as_mut() else {
            return Ok(LoginProgress::default());
        };

        let actions = login.handle(packet)?;
        let progress = self.apply_login_actions(actions);
        if progress.completed.is_some() {
            self.login = None;
        }
        Ok(progress)
    }

    fn apply_login_actions(&mut self, actions: Vec<ServerLoginAction<BedrockProtocol>>) -> LoginProgress {
        let mut progress = LoginProgress::default();
        for action in actions {
            match action {
                LoginAction::Send(packets) => packets.into_iter().for_each(|packet| self.send_immediate(packet)),
                LoginAction::EnableCompression(compression) => {
                    progress.settings_accepted = true;
                    self.set_compression(Some(compression));
                }
                LoginAction::EnableEncryption(encryption) => self.set_encryption(Some(*encryption)),
                LoginAction::Complete(login) => progress.completed = Some(login),
            }
        }
        progress
    }

    pub fn fail_login(&mut self, failure: LoginFailure<BedrockProtocol>) {
        warn!("Rejected login: {}", failure.error);
        failure.farewell.into_iter().for_each(|packet| self.send_immediate(packet));
        self.close(disconnect_message(&failure.error));
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

fn disconnect_message(error: &LoginError) -> Option<&'static str> {
    match error {
        LoginError::ProtocolMismatch { client, server } if client < server => Some("disconnectionScreen.outdatedClient"),
        LoginError::ProtocolMismatch { .. } => Some("disconnectionScreen.outdatedServer"),
        LoginError::Auth(_) => Some("disconnectionScreen.notAuthenticated"),
        LoginError::NotAuthenticated => None,
        _ => Some("disconnectionScreen.noReason"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bedrock::auth::{ClientData, ConnectionRequest};
    use bedrock::network::login::LoginPackets;
    use bedrock::protocol::v662::packets::ClientToServerHandshakePacket;
    use bevy_raknet::prelude::RakSessionId;

    fn session_with_options(options: ServerLoginOptions) -> Session {
        Session::new(Entity::PLACEHOLDER, SessionId::RakNet(RakSessionId(0)), options)
    }

    fn offline_options() -> ServerLoginOptions {
        ServerLoginOptions::default().encryption(false).require_authentication(false)
    }

    fn self_signed_login(name: &str) -> BedrockProtocol {
        let key = p384::SecretKey::from_slice(&[0x11u8; 48]).unwrap();
        let client_data = ClientData::offline("1.21.0", "127.0.0.1:19132", name);
        let request = ConnectionRequest::self_signed(&key, name, &client_data).unwrap();
        BedrockProtocol::login(BedrockProtocol::PROTOCOL_VERSION as i32, request.to_bytes().unwrap())
    }

    fn settings_request(protocol: i32) -> BedrockProtocol {
        BedrockProtocol::request_network_settings(protocol)
    }

    fn session_with_zlib() -> Session {
        let mut session = session_with_options(ServerLoginOptions::default());
        session.compression = Some(Compression::Zlib { threshold: 0, compression_level: 6 });
        session
    }

    fn compressed_batch(packets: usize) -> Vec<u8> {
        let packets = vec![BedrockProtocol::ClientToServerHandshakePacket(ClientToServerHandshakePacket {}.into()); packets];
        encode_packets(&packets, Some(&Compression::Zlib { threshold: 0, compression_level: 6 }), None).unwrap()
    }

    #[test]
    fn decode_returns_the_packets_of_a_batch_within_the_limit() {
        let batch = compressed_batch(100);

        assert_eq!(session_with_zlib().decode(batch, usize::MAX).len(), 100);
    }

    #[test]
    fn decode_drops_a_batch_that_inflates_past_the_limit() {
        let batch = compressed_batch(100);

        assert!(session_with_zlib().decode(batch, 100).is_empty());
    }

    #[test]
    fn offline_login_enables_compression_then_completes() {
        let mut session = session_with_options(offline_options());

        let progress = session.advance_login(settings_request(BedrockProtocol::PROTOCOL_VERSION as i32)).unwrap();
        assert!(progress.settings_accepted);
        assert!(progress.completed.is_none());
        assert_eq!(session.compression, Some(Compression::Zlib { compression_level: 6, threshold: 256 }));
        assert_eq!(session.take_outgoing().len(), 1);

        let progress = session.advance_login(self_signed_login("Steve")).unwrap();
        let login = progress.completed.expect("login completes without encryption");
        assert_eq!(login.authentication.identity().display_name, "Steve");
        assert_eq!(session.take_outgoing().len(), 1);
        assert!(session.login.is_none());
    }

    #[test]
    fn encrypted_login_waits_for_the_client_handshake() {
        let options = ServerLoginOptions::default().require_authentication(false);
        let mut session = session_with_options(options);
        session.advance_login(settings_request(BedrockProtocol::PROTOCOL_VERSION as i32)).unwrap();

        let progress = session.advance_login(self_signed_login("Steve")).unwrap();
        assert!(progress.completed.is_none());
        assert!(session.encryption.is_some());

        let progress = session.advance_login(BedrockProtocol::client_handshake()).unwrap();
        assert!(progress.completed.is_some());
    }

    #[test]
    fn outdated_client_is_told_and_closed() {
        let mut session = session_with_options(offline_options());

        let failure = session
            .advance_login(settings_request(BedrockProtocol::PROTOCOL_VERSION as i32 - 1))
            .err()
            .expect("an older protocol is rejected");
        assert!(matches!(failure.error, LoginError::ProtocolMismatch { .. }));
        assert_eq!(failure.farewell.len(), 1);
        assert_eq!(disconnect_message(&failure.error), Some("disconnectionScreen.outdatedClient"));

        session.fail_login(failure);
        assert!(session.is_closed());
        assert_eq!(session.take_outgoing().len(), 2);
    }

    #[test]
    fn unauthenticated_login_is_rejected_when_authentication_is_required() {
        let mut session = session_with_options(ServerLoginOptions::default().encryption(false));
        session.advance_login(settings_request(BedrockProtocol::PROTOCOL_VERSION as i32)).unwrap();

        let failure = session.advance_login(self_signed_login("Steve")).err().expect("a self-signed login is not authenticated");
        assert!(matches!(failure.error, LoginError::NotAuthenticated));
        assert_eq!(disconnect_message(&failure.error), None);
    }

    #[test]
    fn newer_client_hits_outdated_server_text() {
        let error = LoginError::ProtocolMismatch { client: 2, server: 1 };
        assert_eq!(disconnect_message(&error), Some("disconnectionScreen.outdatedServer"));
    }
}
