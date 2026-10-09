use crate::config::Config;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::login::auth::Auth;
use crate::network::login::encryption::get_handshake_jwt;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::player::identity::PlayerIdentity;
use bedrock::auth::{ChainRoot, ConnectionRequest, Identity};
use bedrock::network::encryption::Encryption;
use bedrock::protocol::v662::enums::PlayStatus;
use bedrock::protocol::v662::packets::ServerToClientHandshakePacket;
use bevy_ecs::message::{Message, MessageReader};
use bevy_ecs::prelude::{Commands, Entity, MessageWriter, Query, Res};
use p384::SecretKey;
use p384::elliptic_curve::Generate;
use rand::RngExt;
use tracing::*;

#[derive(Message, Clone, Debug)]
pub struct PlayerLoginMessage {
    pub entity: Entity,
    pub name: String,
    pub xuid: String,
}

pub fn handle_login(
    config: Res<Config>,
    oidc: Option<Res<Auth>>,
    mut reader: MessageReader<PacketReceivedMessage>,
    mut writer: MessageWriter<SessionStateChangedMessage>,
    mut login_writer: MessageWriter<PlayerLoginMessage>,
    mut sessions: Query<&mut Session>,
    mut commands: Commands,
) {
    for ev in reader.read() {
        let Ok(mut session) = sessions.get_mut(ev.entity) else {
            continue;
        };

        if session.get_state() != SessionState::Login {
            continue;
        }

        let BedrockProtocol::LoginPacket(packet) = &ev.packet else {
            continue;
        };

        let login = match ConnectionRequest::parse(&packet.connection_request).and_then(|request| request.verify(oidc.as_deref().map(|a| &a.0), &ChainRoot::MOJANG)) {
            Ok(login) => login,
            Err(error) => {
                warn!("Rejected login: {error}");
                session.close(Some("disconnectionScreen.notAuthenticated"));
                continue;
            }
        };

        if !login.authentication.is_authenticated() && config.server.authentication {
            session.close(Some("disconnectionScreen.notAuthenticated"));
            continue;
        }

        let identity = login.authentication.identity();

        let handshake = if config.network.raknet.encryption && session.id.supports_encryption() {
            let Some(handshake) = start_encryption(identity) else {
                warn!("Failed to start encryption for {}", identity.display_name);
                session.close(Some("disconnectionScreen.noReason"));
                continue;
            };
            Some(handshake)
        } else {
            None
        };

        commands.entity(ev.entity).insert((
            PlayerIdentity::new(identity.display_name.clone(), identity.xuid.clone()),
            crate::player::skin::PlayerAppearance::from_client_data(&login.client_data),
        ));

        login_writer.write(PlayerLoginMessage {
            entity: ev.entity,
            name: identity.display_name.clone(),
            xuid: identity.xuid.clone(),
        });

        match handshake {
            Some((jwt, encryption)) => {
                session.send_immediate(BedrockProtocol::ServerToClientHandshakePacket(ServerToClientHandshakePacket { handshake_web_token: jwt }.into()));
                session.set_encryption(Some(encryption));
                session.set_state(SessionState::Handshake, &mut writer);
            }
            None => session.set_state(SessionState::Resource, &mut writer),
        }

        session.send_play_status(PlayStatus::LoginSuccess, false);
    }
}

fn start_encryption(identity: &Identity) -> Option<(String, Encryption)> {
    let client_key = identity.public_key().ok()?;

    let mut token = [0u8; 16];
    rand::rng().fill(&mut token);

    let secret = SecretKey::generate();
    let jwt = get_handshake_jwt(&secret, &token)?;

    Some((jwt, Encryption::new(&secret, &client_key, &token)))
}
