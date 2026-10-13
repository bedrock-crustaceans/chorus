use super::{LoginProgress, Session};
use crate::network::BedrockProtocol;
use bedrock::auth::Login;
use bedrock::network::compression::Compression;
use bedrock::network::error::LoginError;
use bedrock::network::login::{LoginOptions, NetworkSettings, begin_encryption, check_protocol};
use bedrock::protocol::ProtoVersion;
use bedrock::protocol::v662::enums::{PacketCompressionAlgorithm, PlayStatus};
use bedrock::protocol::v662::packets::{NetworkSettingsPacket, ServerToClientHandshakePacket};
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
}

impl LoginFlow {
    pub fn new(config: LoginConfig) -> Self {
        Self {
            config,
            state: LoginState::AwaitSettings,
        }
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
