use super::{LoginProgress, Session};
use crate::network::BedrockProtocol;
use bedrock::auth::Login;
use bedrock::network::compression::Compression;
use bedrock::network::error::LoginError;
use bedrock::network::login::{LoginOptions, NetworkSettings, begin_encryption, bind_transport_identity, check_protocol};
use bedrock::protocol::ProtoVersion;
use bedrock::protocol::v662::enums::{PacketCompressionAlgorithm, PlayStatus};
use bedrock::protocol::v662::packets::{NetworkSettingsPacket, ServerToClientHandshakePacket};
use p384::PublicKey;
use std::fmt::{Display, Formatter};
use std::mem::replace;
use tracing::warn;

const PROTOCOL: i32 = BedrockProtocol::PROTOCOL_VERSION as i32;

pub struct LoginConfig {
    pub options: LoginOptions,
    pub settings: NetworkSettings,
    pub encryption: bool,
}

pub struct LoginFlow {
    config: LoginConfig,
    state: LoginState,
    transport_key: Option<PublicKey>,
}

impl LoginFlow {
    pub fn new(config: LoginConfig) -> Self {
        Self {
            config,
            state: LoginState::AwaitSettings,
            transport_key: None,
        }
    }

    pub fn bind_transport_key(&mut self, key: PublicKey) {
        self.transport_key = Some(key);
    }

    pub fn is_done(&self) -> bool {
        matches!(self.state, LoginState::Done)
    }
}

enum LoginState {
    AwaitSettings,
    AwaitLogin,
    AwaitHandshake(Box<Login>),
    Done,
}

impl LoginState {
    fn name(&self) -> &'static str {
        match self {
            Self::AwaitSettings => "awaiting network settings request",
            Self::AwaitLogin => "awaiting login",
            Self::AwaitHandshake(_) => "awaiting client handshake",
            Self::Done => "login finished",
        }
    }
}

#[derive(Debug)]
pub enum LoginFailure {
    Rejected(LoginError),
    UnexpectedPacket { state: &'static str },
}

impl From<LoginError> for LoginFailure {
    fn from(error: LoginError) -> Self {
        Self::Rejected(error)
    }
}

impl Display for LoginFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rejected(error) => error.fmt(f),
            Self::UnexpectedPacket { state } => write!(f, "unexpected packet while {state}"),
        }
    }
}

impl LoginFailure {
    pub fn disconnect_message(&self) -> Option<&'static str> {
        match self {
            Self::Rejected(LoginError::ProtocolMismatch { client, server }) if client < server => Some("disconnectionScreen.outdatedClient"),
            Self::Rejected(LoginError::ProtocolMismatch { .. }) => Some("disconnectionScreen.outdatedServer"),
            Self::Rejected(LoginError::Auth(_) | LoginError::NotAuthenticated) => Some("disconnectionScreen.notAuthenticated"),
            _ => Some("disconnectionScreen.noReason"),
        }
    }
}

impl Session {
    pub fn advance_login(&mut self, packet: BedrockProtocol) -> Result<LoginProgress, LoginFailure> {
        match (replace(&mut self.login.state, LoginState::Done), packet) {
            (LoginState::Done, _) => Ok(LoginProgress::default()),
            (LoginState::AwaitSettings, BedrockProtocol::RequestNetworkSettingsPacket(request)) => self.accept_settings_request(request.client_network_version),
            (LoginState::AwaitLogin, BedrockProtocol::LoginPacket(login)) => self.accept_login(login.client_network_version, &login.connection_request),
            (LoginState::AwaitHandshake(login), BedrockProtocol::ClientToServerHandshakePacket(_)) => Ok(self.finish_login(login)),
            (state, _) => {
                let failure = LoginFailure::UnexpectedPacket { state: state.name() };
                self.login.state = state;
                Err(failure)
            }
        }
    }

    fn accept_settings_request(&mut self, client_protocol: i32) -> Result<LoginProgress, LoginFailure> {
        if let Err(error) = check_protocol(client_protocol, PROTOCOL) {
            let status = if client_protocol < PROTOCOL {
                PlayStatus::LoginFailedClientOld
            } else {
                PlayStatus::LoginFailedServerOld
            };
            self.send_play_status(status, true);
            return Err(error.into());
        }

        let settings = self.login.config.settings.clone();
        let algorithm = match settings.compression {
            Compression::Zlib { .. } => PacketCompressionAlgorithm::ZLib,
            Compression::Snappy { .. } => PacketCompressionAlgorithm::Snappy,
            Compression::None => PacketCompressionAlgorithm::None,
        };
        self.send_immediate(BedrockProtocol::NetworkSettingsPacket(
            NetworkSettingsPacket {
                compression_threshold: settings.threshold(),
                compression_algorithm: algorithm,
                client_throttle_enabled: false,
                client_throttle_threshold: 0,
                client_throttle_scalar: 0.0,
            }
            .into(),
        ));
        self.set_compression(Some(settings.compression));

        self.login.state = LoginState::AwaitLogin;
        Ok(LoginProgress {
            settings_accepted: true,
            completed: None,
        })
    }

    fn accept_login(&mut self, client_protocol: i32, connection_request: &[u8]) -> Result<LoginProgress, LoginFailure> {
        check_protocol(client_protocol, PROTOCOL)?;
        let login = self.login.config.options.verify(connection_request)?;
        if let Some(key) = &self.login.transport_key {
            bind_transport_identity(&login, key)?;
        }

        if !self.login.config.encryption {
            return Ok(self.finish_login(Box::new(login)));
        }

        let (token, encryption) = begin_encryption(&login)?;
        self.send_immediate(BedrockProtocol::ServerToClientHandshakePacket(ServerToClientHandshakePacket { handshake_web_token: token }.into()));
        self.set_encryption(Some(encryption));
        self.login.state = LoginState::AwaitHandshake(Box::new(login));
        Ok(LoginProgress::default())
    }

    fn finish_login(&mut self, login: Box<Login>) -> LoginProgress {
        self.send_play_status(PlayStatus::LoginSuccess, true);
        LoginProgress {
            settings_accepted: false,
            completed: Some(login),
        }
    }

    pub fn fail_login(&mut self, failure: LoginFailure) {
        warn!("Rejected login: {failure}");
        self.close(failure.disconnect_message());
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::network::transport::SessionId;
    use bedrock::auth::{ClientData, ConnectionRequest};
    use bedrock::protocol::v662::packets::{ClientToServerHandshakePacket, LoginPacket, RequestNetworkSettingsPacket};
    use bevy_ecs::prelude::Entity;
    use bevy_raknet::prelude::RakSessionId;

    fn config(encryption: bool, require_authentication: bool) -> LoginConfig {
        LoginConfig {
            options: LoginOptions::default().require_authentication(require_authentication),
            settings: NetworkSettings::new(Compression::Zlib { compression_level: 6, threshold: 256 }),
            encryption,
        }
    }

    pub fn session_with_config(encryption: bool, require_authentication: bool) -> Session {
        Session::new(Entity::PLACEHOLDER, SessionId::RakNet(RakSessionId(0)), config(encryption, require_authentication))
    }

    fn offline_session() -> Session {
        session_with_config(false, false)
    }

    fn self_signed_login(name: &str) -> BedrockProtocol {
        let key = p384::SecretKey::from_slice(&[0x11u8; 48]).unwrap();
        let client_data = ClientData::offline("1.21.0", "127.0.0.1:19132", name);
        let request = ConnectionRequest::self_signed(&key, name, &client_data).unwrap();
        BedrockProtocol::LoginPacket(
            LoginPacket {
                client_network_version: PROTOCOL,
                connection_request: request.to_bytes().unwrap(),
            }
            .into(),
        )
    }

    fn settings_request(protocol: i32) -> BedrockProtocol {
        BedrockProtocol::RequestNetworkSettingsPacket(RequestNetworkSettingsPacket { client_network_version: protocol }.into())
    }

    fn client_handshake() -> BedrockProtocol {
        BedrockProtocol::ClientToServerHandshakePacket(ClientToServerHandshakePacket {}.into())
    }

    #[test]
    fn offline_login_enables_compression_then_completes() {
        let mut session = offline_session();

        let progress = session.advance_login(settings_request(PROTOCOL)).unwrap();
        assert!(progress.settings_accepted);
        assert!(progress.completed.is_none());
        assert_eq!(session.compression, Some(Compression::Zlib { compression_level: 6, threshold: 256 }));
        assert_eq!(session.take_outgoing().len(), 1);

        let progress = session.advance_login(self_signed_login("Steve")).unwrap();
        let login = progress.completed.expect("login completes without encryption");
        assert_eq!(login.authentication.identity().display_name, "Steve");
        assert_eq!(session.take_outgoing().len(), 1);
        assert!(session.login.is_done());
    }

    #[test]
    fn encrypted_login_waits_for_the_client_handshake() {
        let mut session = session_with_config(true, false);
        session.advance_login(settings_request(PROTOCOL)).unwrap();

        let progress = session.advance_login(self_signed_login("Steve")).unwrap();
        assert!(progress.completed.is_none());
        assert!(session.encryption.is_some());

        let progress = session.advance_login(client_handshake()).unwrap();
        assert!(progress.completed.is_some());
    }

    #[test]
    fn outdated_client_is_told_and_closed() {
        let mut session = offline_session();

        let failure = session.advance_login(settings_request(PROTOCOL - 1)).err().expect("an older protocol is rejected");
        assert!(matches!(failure, LoginFailure::Rejected(LoginError::ProtocolMismatch { .. })));
        assert_eq!(failure.disconnect_message(), Some("disconnectionScreen.outdatedClient"));
        assert_eq!(session.take_outgoing().len(), 1);

        session.fail_login(failure);
        assert!(session.is_closed());
        assert_eq!(session.take_outgoing().len(), 1);
    }

    #[test]
    fn unauthenticated_login_is_rejected_when_authentication_is_required() {
        let mut session = session_with_config(false, true);
        session.advance_login(settings_request(PROTOCOL)).unwrap();

        let failure = session.advance_login(self_signed_login("Steve")).err().expect("a self-signed login is not authenticated");
        assert!(matches!(failure, LoginFailure::Rejected(LoginError::NotAuthenticated)));
        assert_eq!(failure.disconnect_message(), Some("disconnectionScreen.notAuthenticated"));
    }

    #[test]
    fn login_from_another_key_than_the_transport_identity_is_rejected() {
        let mut session = offline_session();
        session.login.bind_transport_key(p384::SecretKey::from_slice(&[0x22u8; 48]).unwrap().public_key());
        session.advance_login(settings_request(PROTOCOL)).unwrap();

        let failure = session.advance_login(self_signed_login("Steve")).err().expect("the key differs from the transport identity");
        assert!(matches!(failure, LoginFailure::Rejected(LoginError::TransportIdentityMismatch)));
    }

    #[test]
    fn login_before_network_settings_is_unexpected() {
        let mut session = offline_session();

        let failure = session.advance_login(self_signed_login("Steve")).err().expect("settings come first");
        assert!(matches!(failure, LoginFailure::UnexpectedPacket { .. }));
    }

    #[test]
    fn newer_client_hits_outdated_server_text() {
        let failure = LoginFailure::Rejected(LoginError::ProtocolMismatch { client: 2, server: 1 });
        assert_eq!(failure.disconnect_message(), Some("disconnectionScreen.outdatedServer"));
    }
}
