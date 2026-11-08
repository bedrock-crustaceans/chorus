use bedrock::network::raknet::RakNetGamePacket;
use bedrock::protocol::ProtoVersion;
use bevy_app::{App, Plugin, PostUpdate, PreUpdate, Startup};
use bevy_ecs::prelude::*;
use bevy_raknet::prelude::{RakDisconnectReason, RakPriority, RakReliability, RakServer, RakServerConfig, RakServerEvent, RakServerPlugin, RakServerSet, RakSessionId};
use chorus_core::config::{AppConfigExt, ConfigSection, NetworkConfig, ServerConfig};
use chorus_core::protocol::BedrockProtocol;
use chorus_core::schedule::{Budget, Priority, Scheduler, Wake};
use chorus_network::bandwidth::BandwidthTracker;
use chorus_network::login::auth::Auth;
use chorus_network::login::login_config;
use chorus_network::session::Session;
use chorus_network::transport::{DisconnectSessionMessage, SendBatchMessage, SessionId};
use chorus_network::{ChunkSendTask, NetworkPlugin, NetworkSet, NetworkState, PacketReceivedMessage, motd};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use tracing::{error, info, warn};

const NAME: &str = "raknet";

struct RakNetTask;

#[derive(Default)]
pub struct RakNetPlugin;

#[derive(Resource, Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct RakNetConfig {
    pub encryption: bool,
}

impl ConfigSection for RakNetConfig {
    const NAME: &'static str = "raknet";
}

impl Default for RakNetConfig {
    fn default() -> Self {
        Self { encryption: true }
    }
}

#[derive(Resource, Clone, Copy)]
struct RakNetSettings {
    encryption: bool,
}

impl RakNetSettings {
    fn id(&self, key: RakSessionId) -> SessionId {
        SessionId::new(NAME, key.0.to_string(), self.encryption)
    }
}

impl Plugin for RakNetPlugin {
    fn build(&self, app: &mut App) {
        Scheduler::register::<RakNetTask, _>(app.world_mut(), Priority::Io, pump);

        if !app.is_plugin_added::<NetworkPlugin>() {
            app.add_plugins(NetworkPlugin);
        }

        app.init_config::<RakNetConfig>()
            .add_plugins(RakServerPlugin)
            .add_systems(Startup, bind)
            .add_systems(PreUpdate, (accept, receive, wake_pump).chain().after(RakServerSet))
            .add_systems(PostUpdate, send.after(NetworkSet::Flush));
    }
}

#[tracing::instrument(name = "network", skip_all)]
fn bind(server: Res<ServerConfig>, network: Res<NetworkConfig>, rak: Res<RakNetConfig>, mut commands: Commands) {
    let ip = IpAddr::V4(Ipv4Addr::from_str(network.ip.as_str()).unwrap_or_else(|err| {
        error!("{}: {}", err, network.ip);

        Ipv4Addr::UNSPECIFIED
    }));
    let bind_addr = SocketAddr::new(ip, network.port);
    let guid = rand::random::<u64>();
    let max_players = server.max_players.max(0) as usize;

    let server = RakServer::new(bind_addr, |conf: &mut RakServerConfig| {
        conf.guid = guid;
        conf.protocols = Box::new([BedrockProtocol::RAKNET_VERSION]);
        conf.max_connections = max_players;
        conf.message = motd(&server, guid, bind_addr.port()).into();
    })
    .expect("failed to bind raknet server");

    info!("Listening for RakNet connections on {bind_addr}");
    commands.insert_resource(RakNetSettings { encryption: rak.encryption });
    commands.insert_resource(server);
}

#[tracing::instrument(name = "network", skip_all)]
fn log_disconnect(id: RakSessionId, reason: RakDisconnectReason) {
    match reason {
        RakDisconnectReason::Requested | RakDisconnectReason::Remote => info!("Disconnected: {:?} ({:?})", id, reason),
        RakDisconnectReason::Timeout | RakDisconnectReason::ProtocolViolation | RakDisconnectReason::OutboundBytesExceeded => warn!("Disconnected: {:?} ({:?})", id, reason),
    }
}

fn accept(
    mut events: MessageReader<RakServerEvent>,
    settings: Res<RakNetSettings>,
    server: Res<ServerConfig>,
    network: Res<NetworkConfig>,
    oidc: Option<Res<Auth>>,
    mut state: ResMut<NetworkState>,
    mut commands: Commands,
    mut sessions: Query<&mut Session>,
) {
    let options = |id: &SessionId| login_config(&server, &network, oidc.as_deref(), id);

    for event in events.read() {
        match event {
            RakServerEvent::SessionConnected { id, .. } => state.connect(&mut commands, settings.id(*id), &options),
            RakServerEvent::SessionDisconnected { id, reason } => {
                log_disconnect(*id, *reason);
                state.disconnect(&mut commands, &mut sessions, &settings.id(*id));
            }
        }
    }
}

fn receive(
    server: Option<ResMut<RakServer>>,
    settings: Res<RakNetSettings>,
    network: Res<NetworkConfig>,
    state: Res<NetworkState>,
    bandwidth: Res<BandwidthTracker>,
    mut sessions: Query<&mut Session>,
    mut packets: MessageWriter<PacketReceivedMessage>,
) {
    let Some(mut server) = server else { return };

    let max_batch_size = network.max_batch_size;
    while let Some((id, buf)) = server.recv() {
        let batch = match RakNetGamePacket::unwrap(&buf) {
            Ok(batch) => batch,
            Err(_) => {
                warn!("dropping RakNet datagram with missing/invalid game packet header");
                continue;
            }
        };
        bandwidth.counters().add_received(batch.len() as u64);

        let Some(entity) = state.entity(&settings.id(id)) else { continue };
        let Ok(mut session) = sessions.get_mut(entity) else { continue };

        for packet in session.decode(batch.into(), max_batch_size) {
            packets.write(PacketReceivedMessage { entity, packet });
        }
    }
}

fn wake_pump(mut scheduler: ResMut<Scheduler>) {
    scheduler.wake::<RakNetTask>();
}

fn pump(_budget: In<Budget>, server: Option<ResMut<RakServer>>, mut sessions: Query<&mut Session>, mut scheduler: ResMut<Scheduler>) -> Wake {
    let Some(mut server) = server else { return Wake::Done };
    server.update();

    for mut session in &mut sessions {
        if session.id.transport == NAME
            && let Ok(key) = session.id.key.parse()
        {
            session.set_send_capacity(server.send_capacity(RakSessionId(key)));
        }
    }

    scheduler.wake::<ChunkSendTask>();
    server.next_timeout().map_or(Wake::Done, Wake::after)
}

fn send(server: Option<ResMut<RakServer>>, mut batches: MessageReader<SendBatchMessage>, mut disconnects: MessageReader<DisconnectSessionMessage>) {
    let Some(mut server) = server else { return };

    for message in batches.read().filter(|message| message.id.transport == NAME) {
        let Ok(id) = message.id.key.parse().map(RakSessionId) else { continue };
        let buf = RakNetGamePacket::wrap(&message.batch.data);
        let priority = if message.batch.immediate { RakPriority::Immediate } else { RakPriority::Normal };
        if let Err(err) = server.send(id, buf, RakReliability::ReliableOrdered, priority) {
            warn!("failed to send batch of {} bytes to {id:?}: {err}", message.batch.data.len());
        }
    }

    for message in disconnects.read().filter(|message| message.id.transport == NAME) {
        if let Ok(key) = message.id.key.parse() {
            server.disconnect(RakSessionId(key));
        }
    }
}
