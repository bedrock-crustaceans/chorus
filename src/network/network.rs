use crate::chat::{BroadcastMessage, PlayerChatMessage};
use crate::command::dispatch::{CommandPreprocessMessage, CommandRequestedMessage};
use crate::config::{Config, NetworkTransport};
use crate::item::ItemTakenMessage;
use crate::level::{BlockUpdatedMessage, LevelEventMessage, LevelSoundMessage};
use crate::network::BedrockProtocol;
use crate::network::bandwidth::BandwidthTracker;
use crate::network::handler::form::FormResponseMessage;
use crate::network::handler::inventory::{InventoryCloseMessage, InventoryOpenMessage, ItemDropMessage, PlayerItemHeldMessage};
use crate::network::handler::login::{PlayerLoginMessage, PlayerPreLoginMessage, login_config};
use crate::network::handler::play::{PlayerJoinedMessage, PlayerMoveMessage, PlayerQuitMessage};
use crate::network::handler::resource::ResourcePackResponseMessage;
use crate::network::handler::{PacketHandlers, PacketReceivedMessage};
use crate::network::login::auth::{Auth, LoginAuthOIDC};
use crate::network::session::state::SessionStateChangedMessage;
use crate::network::session::{Batch, LoginConfig, Session};
use crate::network::transport::{SessionId, Transports};
use crate::world::block::{BlockBreakMessage, BlockPlaceMessage};
use crate::world::chunks::ChunkSendTask;
use crate::{Budget, Priority, Scheduler, Tick, TickSet, Wake};
use bedrock::network::info::MINECRAFT_EDITION_MOTD;
use bedrock::network::motd::BedrockMOTD;
use bedrock::network::nethernet::IdentityStore;
use bedrock::protocol::ProtoVersion;
use bevy_app::{App, Plugin, PostUpdate, PreUpdate, Startup};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use bevy_nethernet::prelude::*;
use bevy_raknet::prelude::*;
use bevy_tasks::ComputeTaskPool;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

const LAN_DISCOVERY_PORT: u16 = 7551;
const NETHERNET_IDENTITY_PATH: &str = "nethernet.pem";
const NETHERNET_IDENTITY_RENEWAL: Duration = Duration::from_secs(12 * 60 * 60);

#[derive(Resource)]
struct NetherIdentity(IdentityStore);
use std::str::FromStr;
use tracing::{error, info, warn};

#[derive(Resource, Default)]
pub struct NetworkState {
    sessions: HashMap<SessionId, Entity>,
}

struct ReceiveTask;

struct RakNetTask;

#[derive(SystemParam)]
pub struct ConnectionEvents<'w, 's> {
    rak: MessageReader<'w, 's, RakServerEvent>,
    nether_lan: MessageReader<'w, 's, NetherServerEvent>,
    nether_http: MessageReader<'w, 's, NetherHttpServerEvent>,
}

pub struct Network;

impl Plugin for Network {
    fn build(&self, app: &mut App) {
        Scheduler::register::<ReceiveTask, _>(app.world_mut(), Priority::Io, Network::receive);
        Scheduler::register::<RakNetTask, _>(app.world_mut(), Priority::Io, Network::pump_raknet);

        app.add_plugins((PacketHandlers, crate::world::WorldPlugin, crate::actor::ActorPlugin, crate::chat::ChatPlugin))
            .add_plugins(LoginAuthOIDC)
            .add_plugins(RakServerPlugin)
            .add_plugins(NetherServerPlugin)
            .add_plugins(NetherHttpServerPlugin)
            .add_systems(Startup, Network::init)
            .add_systems(
                PreUpdate,
                // chained (rather than run as one system) so a newly-spawned Session's Commands are
                // applied before the queued `receive` job looks it up - otherwise a packet from a
                // session that connected this same tick would find no entity and get silently dropped
                (Network::accept, Network::queue_receive).chain().after(RakServerSet).after(NetherServerSet).after(NetherHttpServerSet),
            )
            .add_systems(PostUpdate, Network::flush)
            .add_systems(Tick, (BandwidthTracker::sample, Network::renew_identity).in_set(TickSet::Last))
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
            .add_message::<ItemTakenMessage>()
            .add_message::<InventoryOpenMessage>()
            .add_message::<InventoryCloseMessage>()
            .add_message::<PlayerItemHeldMessage>()
            .add_message::<ItemDropMessage>()
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
    fn motd(config: &Config, guid: u64, port: u16) -> BedrockMOTD {
        BedrockMOTD {
            edition: MINECRAFT_EDITION_MOTD.to_owned(),
            name: config.server.name.clone(),
            sub_name: config.server.description.clone(),
            protocol: BedrockProtocol::PROTOCOL_VERSION,
            version: BedrockProtocol::GAME_VERSION.to_string(),
            player_count: 0,
            player_max: config.server.max_players,
            guid,
            game_mode: "Survival".to_string(),
            nintendo_limited: Some(false),
            port_v4: Some(port),
            port_v6: Some(port),
        }
    }

    #[tracing::instrument(name = "network", skip_all)]
    pub fn init(config: Res<Config>, mut commands: Commands) {
        let ip = IpAddr::V4(Ipv4Addr::from_str(config.network.ip.as_str()).unwrap_or_else(|err| {
            error!("{}: {}", err, config.network.ip);

            Ipv4Addr::UNSPECIFIED
        }));
        let bind_addr = SocketAddr::new(ip, config.network.port);

        match config.network.transport {
            NetworkTransport::RakNet => {
                let guid = rand::random::<u64>();

                let server = RakServer::new(bind_addr, |conf: &mut RakServerConfig| {
                    conf.guid = guid;
                    conf.protocols = Box::new([BedrockProtocol::RAKNET_VERSION]);
                    conf.max_connections = config.server.max_players.max(0) as usize;
                    conf.message = Network::motd(&config, guid, bind_addr.port()).into();
                })
                .expect("failed to bind raknet server");

                info!("Listening for RakNet connections on {bind_addr}");
                commands.insert_resource(server);
            }
            NetworkTransport::NetherNet => {
                let network_id = rand::random::<u64>();

                let mut data = ServerData::from(&Network::motd(&config, network_id, bind_addr.port()));
                data.level_name = config.level.name.clone();

                let signaler = config.network.nethernet.signaler;
                if signaler.lan() {
                    let lan_addr = SocketAddr::new(ip, LAN_DISCOVERY_PORT);
                    let mut lan = NetherServer::new(network_id, lan_addr, |_| {}).expect("failed to bind nethernet lan signaler");
                    lan.set_server_data(data.clone());
                    info!("Listening for NetherNet LAN connections on {lan_addr}");
                    commands.insert_resource(lan);
                }
                if signaler.http() {
                    let mut http = NetherHttpServer::bind(bind_addr, |_| {}).expect("failed to bind nethernet http signaler");
                    http.set_server_data(data);
                    let store = IdentityStore::load_or_generate(NETHERNET_IDENTITY_PATH).expect("failed to prepare nethernet identity");
                    http.set_identity(store.identity().expect("nethernet identity was just validated"));
                    commands.insert_resource(NetherIdentity(store));
                    info!("Listening for NetherNet HTTP signaling on {bind_addr}");
                    commands.insert_resource(http);
                }
            }
        }
    }

    /// Spawns/despawns `Session` entities for new connections/disconnections.
    ///
    /// The *ServerPlugin systems (`RakServerSet`/`NethernetServerSet`/`NethernetHttpServerSet`)
    /// already drain their server's connect/disconnect events into `MessageWriter<*ServerEvent>`
    /// each tick, so this reads those messages directly rather than polling the servers again -
    /// by this point their own event queues are already empty.
    pub fn accept(mut events: ConnectionEvents, config: Res<Config>, oidc: Option<Res<Auth>>, mut state: ResMut<NetworkState>, mut commands: Commands) {
        let options = |id: &SessionId| login_config(&config, oidc.as_deref(), id);

        for event in events.rak.read() {
            match event {
                RakServerEvent::SessionConnected { id, .. } => Network::connect(&mut state, &mut commands, SessionId::RakNet(*id), &options),
                RakServerEvent::SessionDisconnected { id, reason } => {
                    Network::log_disconnect(*id, *reason);
                    Network::disconnect(&mut state, &mut commands, &SessionId::RakNet(*id));
                }
            }
        }

        for event in events.nether_lan.read() {
            match event {
                NetherServerEvent::SessionConnected(id) => Network::connect(&mut state, &mut commands, SessionId::NetherNetLan(id.clone()), &options),
                NetherServerEvent::SessionDisconnected(id) => Network::disconnect(&mut state, &mut commands, &SessionId::NetherNetLan(id.clone())),
            }
        }

        for event in events.nether_http.read() {
            match event {
                NetherHttpServerEvent::SessionConnected(id) => Network::connect(&mut state, &mut commands, SessionId::NetherNetHttp(id.clone()), &options),
                NetherHttpServerEvent::SessionDisconnected(id) => Network::disconnect(&mut state, &mut commands, &SessionId::NetherNetHttp(id.clone())),
            }
        }
    }

    fn queue_receive(mut scheduler: ResMut<Scheduler>) {
        scheduler.wake::<ReceiveTask>();
        scheduler.wake::<RakNetTask>();
    }

    fn pump_raknet(_budget: In<Budget>, server: Option<ResMut<RakServer>>, mut scheduler: ResMut<Scheduler>) -> Wake {
        let Some(mut server) = server else { return Wake::Done };
        server.update();
        scheduler.wake::<ChunkSendTask>();
        server.next_timeout().map_or(Wake::Done, Wake::after)
    }

    fn receive(
        _budget: In<Budget>,
        config: Res<Config>,
        mut transport: Transports,
        state: Res<NetworkState>,
        bandwidth: Res<BandwidthTracker>,
        mut query: Query<&mut Session>,
        mut events: MessageWriter<PacketReceivedMessage>,
    ) -> Wake {
        while let Some((id, data)) = transport.recv() {
            bandwidth.counters().add_received(data.len() as u64);

            let Some(&entity) = state.sessions.get(&id) else {
                continue;
            };
            let Ok(mut session) = query.get_mut(entity) else {
                continue;
            };

            for packet in session.decode(data.into_vec(), config.network.max_batch_size) {
                events.write(PacketReceivedMessage { entity, packet });
            }
        }

        Wake::Done
    }

    fn renew_identity(identity: Option<ResMut<NetherIdentity>>, http: Option<ResMut<NetherHttpServer>>) {
        let (Some(mut identity), Some(mut http)) = (identity, http) else { return };
        match identity.0.renew(NETHERNET_IDENTITY_RENEWAL) {
            Some(Ok(renewed)) => http.set_identity(renewed),
            Some(Err(error)) => {
                let _tag = tracing::info_span!("network").entered();
                error!("failed to renew nethernet identity: {error}");
            }
            None => {}
        }
    }

    #[tracing::instrument(name = "network", skip_all)]
    fn log_disconnect(id: RakSessionId, reason: RakDisconnectReason) {
        match reason {
            RakDisconnectReason::Requested | RakDisconnectReason::Remote => info!("Disconnected: {:?} ({:?})", id, reason),
            RakDisconnectReason::Timeout | RakDisconnectReason::ProtocolViolation | RakDisconnectReason::OutboundBytesExceeded => warn!("Disconnected: {:?} ({:?})", id, reason),
        }
    }

    #[tracing::instrument(name = "network", skip_all)]
    fn connect(state: &mut NetworkState, commands: &mut Commands, id: SessionId, options: &dyn Fn(&SessionId) -> LoginConfig) {
        let entity = commands.spawn_empty().id();
        let session = Session::new(entity, id.clone(), options(&id));

        commands.entity(entity).insert(session);

        info!("Connected: {:?}", id);

        state.sessions.insert(id, entity);
    }

    fn disconnect(state: &mut NetworkState, commands: &mut Commands, id: &SessionId) {
        if let Some(entity) = state.sessions.remove(id) {
            commands.entity(entity).despawn();
        }
    }

    /// Pushes everything the handlers queued this tick out, and reaps closed sessions.
    pub fn flush(mut transport: Transports, mut state: ResMut<NetworkState>, bandwidth: Res<BandwidthTracker>, mut query: Query<(Entity, &mut Session)>, mut commands: Commands) {
        let mut sessions: Vec<(Entity, Mut<Session>)> = query.iter_mut().collect();

        let mut busy: Vec<usize> = Vec::new();
        for (index, (_, session)) in sessions.iter().enumerate() {
            if session.has_queued() {
                busy.push(index);
            }
        }
        let mut encoded: HashMap<usize, Vec<Batch>> = HashMap::new();
        if !busy.is_empty() {
            let mut selected: Vec<(usize, &mut Mut<Session>)> = Vec::new();
            for (index, (_, session)) in sessions.iter_mut().enumerate() {
                if busy.binary_search(&index).is_ok() {
                    selected.push((index, session));
                }
            }
            let results = ComputeTaskPool::get().scope(|scope| {
                for (index, session) in selected {
                    scope.spawn(async move { (index, session.take_outgoing()) });
                }
            });
            encoded.extend(results);
        }

        for (index, (entity, mut session)) in sessions.into_iter().enumerate() {
            let batches = match encoded.remove(&index) {
                Some(batches) => batches,
                None => session.take_outgoing(),
            };
            for batch in batches {
                bandwidth.counters().add_sent(batch.data.len() as u64);
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
