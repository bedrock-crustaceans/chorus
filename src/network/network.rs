use crate::command::dispatch::{CommandPreprocessMessage, CommandRequestedMessage};
use crate::config::{Config, NetworkTransport};
use crate::level::{BlockUpdatedMessage, LevelEventMessage, LevelSoundMessage};
use crate::network::BedrockProtocol;
use crate::network::bandwidth::BandwidthTracker;
use crate::network::handler::block::{BlockBreakMessage, BlockPlaceMessage};
use crate::network::handler::chat::{BroadcastMessage, PlayerChatMessage};
use crate::network::handler::form::FormResponseMessage;
use crate::network::handler::inventory::{InventoryCloseMessage, InventoryOpenMessage, PlayerItemHeldMessage};
use crate::network::handler::login::PlayerLoginMessage;
use crate::network::handler::play::{PlayerJoinedMessage, PlayerMoveMessage, PlayerQuitMessage};
use crate::network::handler::request::PlayerPreLoginMessage;
use crate::network::handler::resource::ResourcePackResponseMessage;
use crate::network::handler::{PacketHandlers, PacketReceivedMessage};
use crate::network::login::auth::LoginAuthOIDC;
use crate::network::session::Session;
use crate::network::session::state::SessionStateChangedMessage;
use crate::network::transport::{ActiveTransport, SessionId};
use bedrock::network::info::MINECRAFT_EDITION_MOTD;
use bedrock::network::motd::BedrockMOTD;
use bedrock::protocol::ProtoVersion;
use bevy_app::{App, Last, Plugin, PostUpdate, PreUpdate, Startup};
use bevy_ecs::prelude::*;
use bevy_nethernet::prelude::*;
use bevy_raknet::prelude::*;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use tracing::{error, info};

#[derive(Resource, Default)]
pub struct NetworkState {
    sessions: HashMap<SessionId, Entity>,
}

pub struct Network;

impl Plugin for Network {
    fn build(&self, app: &mut App) {
        app.add_plugins(PacketHandlers)
            .add_plugins(LoginAuthOIDC)
            .add_plugins(RakServerPlugin)
            .add_plugins(NethernetServerPlugin)
            .add_plugins(NethernetHttpServerPlugin)
            .add_systems(Startup, Network::init)
            .add_systems(
                PreUpdate,
                // chained (rather than run as one system) so a newly-spawned Session's Commands are
                // applied before `receive` looks it up - otherwise a packet from a session that
                // connected this same tick would find no entity and get silently dropped
                (Network::accept, Network::receive).chain().after(RakServerSet).after(NethernetServerSet).after(NethernetHttpServerSet),
            )
            .add_systems(PostUpdate, Network::flush)
            .add_systems(Last, BandwidthTracker::sample)
            .init_resource::<BandwidthTracker>()
            .init_resource::<NetworkState>()
            .add_message::<PacketReceivedMessage>()
            .add_message::<SessionStateChangedMessage>()
            .add_message::<PlayerJoinedMessage>()
            .add_message::<PlayerQuitMessage>()
            .add_message::<PlayerLoginMessage>()
            .add_message::<PlayerPreLoginMessage>()
            .add_message::<PlayerMoveMessage>()
            .add_message::<ResourcePackResponseMessage>()
            .add_message::<BlockUpdatedMessage>()
            .add_message::<BlockBreakMessage>()
            .add_message::<BlockPlaceMessage>()
            .add_message::<InventoryOpenMessage>()
            .add_message::<InventoryCloseMessage>()
            .add_message::<PlayerItemHeldMessage>()
            .add_message::<FormResponseMessage>()
            .add_message::<PlayerChatMessage>()
            .add_message::<CommandPreprocessMessage>()
            .add_message::<CommandRequestedMessage>()
            .add_message::<BroadcastMessage>()
            .add_message::<LevelEventMessage>()
            .add_message::<LevelSoundMessage>();
    }
}

impl Network {
    pub fn init(config: Res<Config>, mut commands: Commands) {
        let ip = IpAddr::V4(Ipv4Addr::from_str(config.ip.as_str()).unwrap_or_else(|err| {
            error!("{}: {}", err, config.ip);

            Ipv4Addr::UNSPECIFIED
        }));
        let bind_addr = SocketAddr::new(ip, config.port);

        match config.transport {
            NetworkTransport::RakNet => {
                let guid = rand::random::<u64>();

                let server = RakServer::new(bind_addr, |conf: &mut RakServerConfig| {
                    conf.guid = guid;
                    conf.protocols = Box::new([BedrockProtocol::RAKNET_VERSION]);
                    conf.max_connections = config.max_players.max(0) as usize;
                    conf.message = BedrockMOTD {
                        edition: MINECRAFT_EDITION_MOTD.to_owned(),
                        name: config.name.clone(),
                        sub_name: config.sub_name.clone(),
                        protocol: BedrockProtocol::PROTOCOL_VERSION,
                        version: BedrockProtocol::GAME_VERSION.to_string(),
                        player_count: 0,
                        player_max: config.max_players,
                        guid,
                        game_mode: "Survival".to_string(),
                        nintendo_limited: Some(false),
                        port_v4: Some(bind_addr.port()),
                        port_v6: Some(bind_addr.port()),
                    }
                    .into();
                })
                .expect("failed to bind raknet server");

                info!("Listening for RakNet connections on {bind_addr}");
                commands.insert_resource(server);
            }
            NetworkTransport::NetherNet => {
                let network_id = rand::random::<u64>();

                let mut data = ServerData::new(config.name.clone(), config.level_name.clone());
                data.max_player_count = config.max_players;
                data.protocol_version = BedrockProtocol::PROTOCOL_VERSION;
                data.game_version = BedrockProtocol::GAME_VERSION.to_string();

                let mut lan = NethernetServer::new(network_id, bind_addr, |_| {}).expect("failed to bind nethernet lan signaler");
                lan.set_server_data(data.clone());

                let http_addr = SocketAddr::new(ip, config.nethernet_http_port);
                let mut http = NethernetHttpServer::bind(http_addr, |_| {}).expect("failed to bind nethernet http signaler");
                http.set_server_data(data);

                info!("Listening for NetherNet connections on {bind_addr} (LAN) and {http_addr} (HTTP signaling)");
                commands.insert_resource(lan);
                commands.insert_resource(http);
            }
        }
    }

    /// Spawns/despawns `Session` entities for new connections/disconnections.
    ///
    /// The *ServerPlugin systems (`RakServerSet`/`NethernetServerSet`/`NethernetHttpServerSet`)
    /// already drain their server's connect/disconnect events into `MessageWriter<*ServerEvent>`
    /// each tick, so this reads those messages directly rather than polling the servers again -
    /// by this point their own event queues are already empty.
    pub fn accept(
        mut rak_events: MessageReader<RakServerEvent>,
        mut nether_lan_events: MessageReader<NethernetServerEvent>,
        mut nether_http_events: MessageReader<NethernetHttpServerEvent>,
        mut state: ResMut<NetworkState>,
        mut commands: Commands,
    ) {
        for event in rak_events.read() {
            match event {
                RakServerEvent::SessionConnected { id, .. } => Network::connect(&mut state, &mut commands, SessionId::RakNet(*id)),
                RakServerEvent::SessionDisconnected { id, .. } => Network::disconnect(&mut state, &mut commands, &SessionId::RakNet(*id)),
            }
        }

        for event in nether_lan_events.read() {
            match event {
                NethernetServerEvent::SessionConnected(id) => Network::connect(&mut state, &mut commands, SessionId::NetherNetLan(id.clone())),
                NethernetServerEvent::SessionDisconnected(id) => Network::disconnect(&mut state, &mut commands, &SessionId::NetherNetLan(id.clone())),
            }
        }

        for event in nether_http_events.read() {
            match event {
                NethernetHttpServerEvent::SessionConnected(id) => Network::connect(&mut state, &mut commands, SessionId::NetherNetHttp(id.clone())),
                NethernetHttpServerEvent::SessionDisconnected(id) => Network::disconnect(&mut state, &mut commands, &SessionId::NetherNetHttp(id.clone())),
            }
        }
    }

    /// Routes inbound datagrams into their `Session`. Runs after [`Network::accept`] (with its
    /// Commands applied in between) so a session that connected this same tick already has one.
    pub fn receive(
        mut rak_server: Option<ResMut<RakServer>>,
        mut nether_lan: Option<ResMut<NethernetServer>>,
        mut nether_http: Option<ResMut<NethernetHttpServer>>,
        state: Res<NetworkState>,
        bandwidth: Res<BandwidthTracker>,
        mut query: Query<&mut Session>,
        mut events: MessageWriter<PacketReceivedMessage>,
    ) {
        let Some(mut transport) = ActiveTransport::from_resources(&mut rak_server, &mut nether_lan, &mut nether_http) else {
            return;
        };

        while let Some((id, data)) = transport.recv() {
            bandwidth.counters().add_received(data.len() as u64);

            let Some(&entity) = state.sessions.get(&id) else {
                continue;
            };
            let Ok(mut session) = query.get_mut(entity) else {
                continue;
            };

            for packet in session.decode(data.into_vec()) {
                events.write(PacketReceivedMessage { entity, packet });
            }
        }
    }

    fn connect(state: &mut NetworkState, commands: &mut Commands, id: SessionId) {
        let entity = commands.spawn_empty().id();
        commands.entity(entity).insert(Session::new(entity, id.clone()));

        info!("Connected: {:?}", id);

        state.sessions.insert(id, entity);
    }

    fn disconnect(state: &mut NetworkState, commands: &mut Commands, id: &SessionId) {
        if let Some(entity) = state.sessions.remove(id) {
            commands.entity(entity).despawn();
        }
    }

    /// Pushes everything the handlers queued this tick out, and reaps closed sessions.
    pub fn flush(
        mut rak_server: Option<ResMut<RakServer>>,
        mut nether_lan: Option<ResMut<NethernetServer>>,
        mut nether_http: Option<ResMut<NethernetHttpServer>>,
        mut state: ResMut<NetworkState>,
        bandwidth: Res<BandwidthTracker>,
        mut query: Query<(Entity, &mut Session)>,
        mut commands: Commands,
    ) {
        let Some(mut transport) = ActiveTransport::from_resources(&mut rak_server, &mut nether_lan, &mut nether_http) else {
            return;
        };

        for (entity, mut session) in query.iter_mut() {
            for batch in session.take_outgoing() {
                bandwidth.counters().add_sent(batch.len() as u64);
                transport.send(&session.id, batch);
            }

            if session.is_closed() {
                transport.disconnect(&session.id);
                state.sessions.remove(&session.id);
                commands.entity(entity).despawn();
            }
        }
    }
}
