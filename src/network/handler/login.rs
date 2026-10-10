use crate::config::Config;
use crate::network::handler::PacketReceivedMessage;
use crate::network::login::auth::Auth;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::network::transport::SessionId;
use crate::player::identity::PlayerIdentity;
use crate::player::skin::PlayerAppearance;
use bedrock::network::compression::Compression;
use bedrock::network::login::server::ServerLoginOptions;
use bevy_ecs::message::{Message, MessageReader};
use bevy_ecs::prelude::{Commands, Entity, MessageWriter, Query};
use tracing::error;

#[derive(Message, Clone, Debug)]
pub struct PlayerPreLoginMessage {
    pub entity: Entity,
}

#[derive(Message, Clone, Debug)]
pub struct PlayerLoginMessage {
    pub entity: Entity,
    pub name: String,
    pub xuid: String,
}

pub fn login_options(config: &Config, oidc: Option<&Auth>, id: &SessionId) -> ServerLoginOptions {
    let options = ServerLoginOptions::default()
        .compression(Compression::Zlib { compression_level: 6, threshold: 1 })
        .encryption(config.network.raknet.encryption && id.supports_encryption())
        .require_authentication(config.server.authentication);

    match oidc {
        Some(auth) => options.oidc(auth.0.clone()),
        None => options,
    }
}

pub fn handle_login(
    mut reader: MessageReader<PacketReceivedMessage>,
    mut writer: MessageWriter<SessionStateChangedMessage>,
    mut pre_login_writer: MessageWriter<PlayerPreLoginMessage>,
    mut login_writer: MessageWriter<PlayerLoginMessage>,
    mut sessions: Query<&mut Session>,
    mut commands: Commands,
) {
    for ev in reader.read() {
        let Ok(mut session) = sessions.get_mut(ev.entity) else {
            error!("received PacketReceivedMessage from entity without a Session!");
            continue;
        };

        if session.get_state() != SessionState::Negotiating || session.is_closed() {
            continue;
        }

        let progress = match session.advance_login(ev.packet.clone()) {
            Ok(progress) => progress,
            Err(failure) => {
                session.fail_login(failure);
                continue;
            }
        };

        if progress.settings_accepted {
            pre_login_writer.write(PlayerPreLoginMessage { entity: ev.entity });
        }

        if let Some(login) = progress.completed {
            let identity = login.authentication.identity();

            commands.entity(ev.entity).insert((
                PlayerIdentity::new(identity.display_name.clone(), identity.xuid.clone()),
                PlayerAppearance::from_client_data(&login.client_data),
            ));

            login_writer.write(PlayerLoginMessage {
                entity: ev.entity,
                name: identity.display_name.clone(),
                xuid: identity.xuid.clone(),
            });

            session.set_state(SessionState::Resource, &mut writer);
        }
    }
}
