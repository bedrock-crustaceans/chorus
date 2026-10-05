use crate::chat::{BroadcastMessage, PlayerChatMessage};
use crate::command::dispatch::{CommandPreprocessMessage, CommandRequestedMessage};
use crate::config::{Config, NetworkTransport};
use crate::item::ItemTakenMessage;
use crate::level::{BlockUpdatedMessage, LevelEventMessage, LevelSoundMessage};
use crate::network::BedrockProtocol;
use crate::network::bandwidth::BandwidthTracker;
use crate::network::handler::form::FormResponseMessage;
use crate::network::handler::inventory::{InventoryCloseMessage, InventoryOpenMessage, ItemDropMessage, PlayerItemHeldMessage};
use crate::network::handler::login::PlayerLoginMessage;
use crate::network::handler::play::{PlayerJoinedMessage, PlayerMoveMessage, PlayerQuitMessage};
use crate::network::handler::request::PlayerPreLoginMessage;
use crate::network::handler::resource::ResourcePackResponseMessage;
use crate::network::handler::{PacketHandlers, PacketReceivedMessage};
use crate::network::login::auth::LoginAuthOIDC;
use crate::network::session::Session;
use crate::network::session::state::SessionStateChangedMessage;
use crate::network::transport::{SessionId, Transports};
use crate::world::block::{BlockBreakMessage, BlockPlaceMessage};
use crate::{JobQueue, Tick, TickSet};
use bedrock::network::info::MINECRAFT_EDITION_MOTD;
use bedrock::network::motd::BedrockMOTD;
use bedrock::protocol::ProtoVersion;
use bevy_app::{App, Plugin, PostUpdate, PreUpdate, Startup};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemId;
use bevy_nethernet::prelude::*;
use bevy_raknet::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant, SystemTime};

const LAN_DISCOVERY_PORT: u16 = 7551;
const NETHERNET_IDENTITY_PATH: &str = "nethernet.pem";
const NETHERNET_IDENTITY_RENEWAL: Duration = Duration::from_secs(12 * 60 * 60);

#[derive(Resource)]
struct NetherIdentity {
    pem: String,
    issued: Instant,
}
use std::str::FromStr;
use tracing::{error, info, warn};

#[derive(Resource, Default)]
pub struct NetworkState {
    sessions: HashMap<SessionId, Entity>,
}

#[derive(Resource)]
struct ReceiveJob(SystemId);

pub struct Network;

impl Plugin for Network {
    fn build(&self, app: &mut App) {
        let receive_job = app.world_mut().register_system(Network::receive);
        app.insert_resource(ReceiveJob(receive_job));

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
                    conf.message = BedrockMOTD {
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

                let mut data = ServerData::new(config.server.name.clone(), config.level.name.clone());
                data.max_player_count = config.server.max_players;
                data.protocol_version = BedrockProtocol::PROTOCOL_VERSION;
                data.game_version = BedrockProtocol::GAME_VERSION.to_string();

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
                    let pem = Network::nethernet_identity_pem();
                    http.set_identity(ServerIdentity::from_pem(&pem, "", SystemTime::now()).expect("nethernet identity was just validated"));
                    commands.insert_resource(NetherIdentity { pem, issued: Instant::now() });
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
    pub fn accept(
        mut rak_events: MessageReader<RakServerEvent>,
        mut nether_lan_events: MessageReader<NetherServerEvent>,
        mut nether_http_events: MessageReader<NetherHttpServerEvent>,
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
                NetherServerEvent::SessionConnected(id) => Network::connect(&mut state, &mut commands, SessionId::NetherNetLan(id.clone())),
                NetherServerEvent::SessionDisconnected(id) => Network::disconnect(&mut state, &mut commands, &SessionId::NetherNetLan(id.clone())),
            }
        }

        for event in nether_http_events.read() {
            match event {
                NetherHttpServerEvent::SessionConnected(id) => Network::connect(&mut state, &mut commands, SessionId::NetherNetHttp(id.clone())),
                NetherHttpServerEvent::SessionDisconnected(id) => Network::disconnect(&mut state, &mut commands, &SessionId::NetherNetHttp(id.clone())),
            }
        }
    }

    fn queue_receive(job: Res<ReceiveJob>, mut jobs: ResMut<JobQueue>) {
        jobs.push(job.0);
    }

    fn receive(
        mut transport: Transports,
        state: Res<NetworkState>,
        bandwidth: Res<BandwidthTracker>,
        mut query: Query<&mut Session>,
        mut events: MessageWriter<PacketReceivedMessage>,
        job: Res<ReceiveJob>,
        mut jobs: ResMut<JobQueue>,
    ) {
        let mut any = false;
        while let Some((id, data)) = transport.recv() {
            any = true;
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

        if any {
            jobs.push(job.0);
        }
    }

    fn nethernet_identity_pem() -> String {
        if let Ok(pem) = fs::read_to_string(NETHERNET_IDENTITY_PATH) {
            match ServerIdentity::from_pem(&pem, "", SystemTime::now()) {
                Ok(_) => return pem,
                Err(error) => warn!("ignoring unreadable {NETHERNET_IDENTITY_PATH}: {error}"),
            }
        }
        let pem = ServerIdentity::generate("", SystemTime::now())
            .and_then(|identity| identity.to_pem())
            .expect("failed to generate nethernet identity");
        if let Err(error) = fs::write(NETHERNET_IDENTITY_PATH, &pem) {
            warn!("failed to save {NETHERNET_IDENTITY_PATH}, clients will be asked to trust a new key next start: {error}");
        }
        pem
    }

    fn renew_identity(identity: Option<ResMut<NetherIdentity>>, http: Option<ResMut<NetherHttpServer>>) {
        let (Some(mut identity), Some(mut http)) = (identity, http) else { return };
        if identity.issued.elapsed() < NETHERNET_IDENTITY_RENEWAL {
            return;
        }
        match ServerIdentity::from_pem(&identity.pem, "", SystemTime::now()) {
            Ok(renewed) => {
                http.set_identity(renewed);
                identity.issued = Instant::now();
            }
            Err(error) => error!("failed to renew nethernet identity: {error}"),
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
    pub fn flush(mut transport: Transports, mut state: ResMut<NetworkState>, bandwidth: Res<BandwidthTracker>, mut query: Query<(Entity, &mut Session)>, mut commands: Commands) {
        for (entity, mut session) in query.iter_mut() {
            for batch in session.take_outgoing() {
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
