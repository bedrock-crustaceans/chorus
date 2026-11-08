use crate::bandwidth::BandwidthTracker;
use crate::login::auth::LoginAuthOIDC;
use crate::session::state::{SessionState, SessionStateChangedMessage};
use crate::session::{LoginConfig, Session};
use crate::transport::{DisconnectSessionMessage, SendBatchMessage, SessionId};
use bedrock::network::info::MINECRAFT_EDITION_MOTD;
use bedrock::network::motd::BedrockMOTD;
use bedrock::protocol::ProtoVersion;
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use chorus_core::config::ServerConfig;
use chorus_core::protocol::BedrockProtocol;
use std::collections::HashMap;
use tracing::info;

#[derive(Message)]
pub struct PacketReceivedMessage {
    pub entity: Entity,
    pub packet: BedrockProtocol,
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NetworkSet {
    Flush,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LoginAuthOIDC)
            .init_resource::<BandwidthTracker>()
            .init_resource::<NetworkState>()
            .add_message::<SendBatchMessage>()
            .add_message::<DisconnectSessionMessage>()
            .add_message::<PacketReceivedMessage>()
            .add_message::<SessionStateChangedMessage>();
    }
}

#[derive(Resource, Default)]
pub struct NetworkState {
    sessions: HashMap<SessionId, Entity>,
}

impl NetworkState {
    pub fn entity(&self, id: &SessionId) -> Option<Entity> {
        self.sessions.get(id).copied()
    }

    pub fn forget(&mut self, id: &SessionId) {
        self.sessions.remove(id);
    }

    #[tracing::instrument(name = "network", skip_all)]
    pub fn connect(&mut self, commands: &mut Commands, id: SessionId, options: &dyn Fn(&SessionId) -> LoginConfig) {
        let entity = commands.spawn_empty().id();
        let session = Session::new(entity, id.clone(), options(&id));

        commands.entity(entity).insert(session);

        info!("Connected: {:?}", id);

        self.sessions.insert(id, entity);
    }

    pub fn disconnect(&mut self, commands: &mut Commands, sessions: &mut Query<&mut Session>, id: &SessionId) {
        let Some(entity) = self.sessions.remove(id) else { return };
        match sessions.get_mut(entity) {
            Ok(mut session) if session.get_state() == SessionState::Play => session.close(None),
            _ => commands.entity(entity).despawn(),
        }
    }
}

pub fn motd(server: &ServerConfig, guid: u64, port: u16) -> BedrockMOTD {
    BedrockMOTD {
        edition: MINECRAFT_EDITION_MOTD.to_owned(),
        name: server.name.clone(),
        sub_name: server.description.clone(),
        protocol: BedrockProtocol::PROTOCOL_VERSION,
        version: BedrockProtocol::GAME_VERSION.to_string(),
        player_count: 0,
        player_max: server.max_players,
        guid,
        game_mode: "Survival".to_string(),
        nintendo_limited: Some(false),
        port_v4: Some(port),
        port_v6: Some(port),
    }
}
