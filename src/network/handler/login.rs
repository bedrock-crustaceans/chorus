use crate::config::Config;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::login::auth::Auth;
use crate::network::login::encryption::get_handshake_jwt;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::player::identity::PlayerIdentity;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bedrock::auth::auth_identity::{AuthData, AuthDataClaims};
use bedrock::network::encryption::Encryption;
use bedrock::protocol::ProtoCodecLE;
use bedrock::protocol::v662::enums::PlayStatus;
use bedrock::protocol::v662::packets::ServerToClientHandshakePacket;
use bevy_ecs::message::{Message, MessageReader};
use bevy_ecs::prelude::{Commands, Entity, MessageWriter, Query, Res};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use p384::elliptic_curve::Generate;
use p384::pkcs8::DecodePublicKey;
use p384::{PublicKey, SecretKey};
use rand::RngExt;
use std::io::Read;
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

        let Some(request) = decode_request(&mut packet.connection_request.as_slice(), oidc.as_deref()) else {
            continue;
        };

        if !request.online && config.online_mode {
            session.close(Some("disconnectionScreen.notAuthenticated"));
            continue;
        }

        commands.entity(ev.entity).insert(PlayerIdentity::new(request.auth_data.xname.clone(), request.auth_data.xid.clone()));

        login_writer.write(PlayerLoginMessage {
            entity: ev.entity,
            name: request.auth_data.xname.clone(),
            xuid: request.auth_data.xid.clone(),
        });

        if config.encryption {
            let mut token = [0u8; 16];
            rand::rng().fill(&mut token);

            let secret = SecretKey::generate();

            let Some(jwt) = get_handshake_jwt(&secret, &token) else {
                warn!("Failed to generate handshake JWT");
                session.close(Some("disconnectionScreen.noReason"));
                continue;
            };

            session.send_immediate(BedrockProtocol::ServerToClientHandshakePacket(ServerToClientHandshakePacket { handshake_web_token: jwt }.into()));

            session.set_encryption(Some(Encryption::new(&secret, &request.key, &token)));

            session.set_state(SessionState::Handshake, &mut writer);
        } else {
            session.set_state(SessionState::Resource, &mut writer);
        }

        session.send_play_status(PlayStatus::LoginSuccess, false);
    }
}

#[derive(Clone, Debug)]
pub struct RequestData {
    online: bool,
    key: PublicKey,
    auth_data: AuthDataClaims,
    client_data: serde_json::Value,
}

fn decode_request<R: Read>(stream: &mut R, oidc: Option<&Auth>) -> Option<RequestData> {
    let auth_data_buf = {
        let len = <i32 as ProtoCodecLE>::deserialize(stream).ok()?;
        let mut buf = vec![0u8; len as usize];
        stream.read_exact(&mut buf).ok()?;
        buf
    };
    let auth_data = serde_json::from_slice::<AuthData>(&auth_data_buf).ok()?;

    let (online, claims) = auth_data.validate(oidc.map(|v| &v.0)).ok()?;

    let der = BASE64_STANDARD.decode(&claims.cpk).ok()?;
    let key = PublicKey::from_public_key_der(&der).ok()?;

    let client_data_buf = {
        let len = <i32 as ProtoCodecLE>::deserialize(stream).ok()?;
        let mut buf = vec![0u8; len as usize];
        stream.read_exact(&mut buf).ok()?;
        buf
    };

    // we use sec1 because DecodingKey uses `from_sec1_bytes` internally instead of `from_public_key_der`. misleading method name
    let dec_key = DecodingKey::from_ec_der(&key.to_sec1_bytes());

    let mut validator = Validation::new(Algorithm::ES384);
    validator.required_spec_claims.remove("exp");
    validator.validate_exp = false;

    let data = decode::<serde_json::Value>(&client_data_buf, &dec_key, &validator).ok()?;

    Some(RequestData {
        online,
        key,
        auth_data: claims,
        client_data: data.claims,
    })
}
