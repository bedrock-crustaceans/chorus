use bedrock::network::nethernet::IdentityStore;
use bevy_app::{App, Plugin, PostUpdate, PreUpdate, Startup};
use bevy_ecs::prelude::*;
use bevy_nethernet::prelude::*;
use chorus_core::config::{AppConfigExt, ConfigSection, LevelConfig, NetworkConfig, ServerConfig};
use chorus_core::schedule::{Tick, TickSet};
use chorus_network::bandwidth::BandwidthTracker;
use chorus_network::login::auth::Auth;
use chorus_network::login::login_config;
use chorus_network::session::Session;
use chorus_network::transport::{DisconnectSessionMessage, SendBatchMessage, SessionId};
use chorus_network::{NetworkPlugin, NetworkSet, NetworkState, PacketReceivedMessage, motd};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, info};

const LAN_DISCOVERY_PORT: u16 = 7551;
const IDENTITY_PATH: &str = "nethernet.pem";
const IDENTITY_RENEWAL: Duration = Duration::from_secs(12 * 60 * 60);
const LAN: &str = "nethernet-lan";
const HTTP: &str = "nethernet-http";

pub struct NetherNetPlugin;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Signaler {
    #[default]
    Http,
    Lan,
    Both,
}

impl Signaler {
    pub fn lan(self) -> bool {
        matches!(self, Self::Lan | Self::Both)
    }

    pub fn http(self) -> bool {
        matches!(self, Self::Http | Self::Both)
    }
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default, deny_unknown_fields)]
pub struct NetherNetConfig {
    pub signaler: Signaler,
}

impl ConfigSection for NetherNetConfig {
    const NAME: &'static str = "nethernet";
}

#[derive(Resource)]
struct NetherIdentity(IdentityStore);

impl Plugin for NetherNetPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<NetworkPlugin>() {
            app.add_plugins(NetworkPlugin);
        }

        app.init_config::<NetherNetConfig>()
            .add_plugins((NetherServerPlugin, NetherHttpServerPlugin))
            .add_systems(Startup, bind)
            .add_systems(PreUpdate, (accept, receive).chain().after(NetherServerSet).after(NetherHttpServerSet))
            .add_systems(PostUpdate, send.after(NetworkSet::Flush))
            .add_systems(Tick, renew_identity.in_set(TickSet::Last));
    }
}

fn session_id(transport: &'static str, id: &NetherSessionId) -> SessionId {
    SessionId::new(transport, format!("{}/{}", id.network_id, id.connection_id), false)
}

fn nether_id(id: &SessionId) -> Option<NetherSessionId> {
    let (network_id, connection_id) = id.key.rsplit_once('/')?;
    Some(NetherSessionId {
        network_id: network_id.to_owned(),
        connection_id: connection_id.parse().ok()?,
    })
}

#[tracing::instrument(name = "network", skip_all)]
fn bind(server: Res<ServerConfig>, network: Res<NetworkConfig>, level: Res<LevelConfig>, nethernet: Res<NetherNetConfig>, mut commands: Commands) {
    let ip = IpAddr::V4(Ipv4Addr::from_str(network.ip.as_str()).unwrap_or_else(|err| {
        error!("{}: {}", err, network.ip);

        Ipv4Addr::UNSPECIFIED
    }));
    let bind_addr = SocketAddr::new(ip, network.port);
    let network_id = rand::random::<u64>();

    let mut data = ServerData::from(&motd(&server, network_id, bind_addr.port()));
    data.level_name = level.name.clone();

    let signaler = nethernet.signaler;
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
        let store = IdentityStore::load_or_generate(IDENTITY_PATH).expect("failed to prepare nethernet identity");
        http.set_identity(store.identity().expect("nethernet identity was just validated"));
        commands.insert_resource(NetherIdentity(store));
        info!("Listening for NetherNet HTTP signaling on {bind_addr}");
        commands.insert_resource(http);
    }
}

fn accept(
    mut lan_events: MessageReader<NetherServerEvent>,
    mut http_events: MessageReader<NetherHttpServerEvent>,
    server: Res<ServerConfig>,
    network: Res<NetworkConfig>,
    oidc: Option<Res<Auth>>,
    mut state: ResMut<NetworkState>,
    mut commands: Commands,
    mut sessions: Query<&mut Session>,
) {
    let options = |id: &SessionId| login_config(&server, &network, oidc.as_deref(), id);

    for event in lan_events.read() {
        match event {
            NetherServerEvent::SessionConnected(id) => state.connect(&mut commands, session_id(LAN, id), &options),
            NetherServerEvent::SessionDisconnected(id) => state.disconnect(&mut commands, &mut sessions, &session_id(LAN, id)),
        }
    }

    for event in http_events.read() {
        match event {
            NetherHttpServerEvent::SessionConnected(id) => state.connect(&mut commands, session_id(HTTP, id), &options),
            NetherHttpServerEvent::SessionDisconnected(id) => state.disconnect(&mut commands, &mut sessions, &session_id(HTTP, id)),
        }
    }
}

fn receive(
    lan: Option<ResMut<NetherServer>>,
    http: Option<ResMut<NetherHttpServer>>,
    network: Res<NetworkConfig>,
    state: Res<NetworkState>,
    bandwidth: Res<BandwidthTracker>,
    mut sessions: Query<&mut Session>,
    mut packets: MessageWriter<PacketReceivedMessage>,
) {
    let mut received: Vec<(SessionId, Box<[u8]>)> = Vec::new();
    if let Some(mut lan) = lan {
        while let Some((id, data)) = lan.recv() {
            received.push((session_id(LAN, &id), data));
        }
    }
    if let Some(mut http) = http {
        while let Some((id, data)) = http.recv() {
            received.push((session_id(HTTP, &id), data));
        }
    }

    let max_batch_size = network.max_batch_size;
    for (id, data) in received {
        bandwidth.counters().add_received(data.len() as u64);

        let Some(entity) = state.entity(&id) else { continue };
        let Ok(mut session) = sessions.get_mut(entity) else { continue };

        for packet in session.decode(data.into_vec(), max_batch_size) {
            packets.write(PacketReceivedMessage { entity, packet });
        }
    }
}

fn send(lan: Option<ResMut<NetherServer>>, http: Option<ResMut<NetherHttpServer>>, mut batches: MessageReader<SendBatchMessage>, mut disconnects: MessageReader<DisconnectSessionMessage>) {
    let (mut lan, mut http) = (lan, http);

    for message in batches.read() {
        let Some(id) = nether_id(&message.id) else { continue };
        match message.id.transport {
            LAN => {
                if let Some(server) = lan.as_mut() {
                    let _ = server.send(&id, &message.batch.data);
                }
            }
            HTTP => {
                if let Some(server) = http.as_mut() {
                    let _ = server.send(&id, &message.batch.data);
                }
            }
            _ => {}
        }
    }

    for message in disconnects.read() {
        let Some(id) = nether_id(&message.id) else { continue };
        match message.id.transport {
            LAN => {
                if let Some(server) = lan.as_mut() {
                    server.disconnect(&id);
                }
            }
            HTTP => {
                if let Some(server) = http.as_mut() {
                    server.disconnect(&id);
                }
            }
            _ => {}
        }
    }
}

fn renew_identity(identity: Option<ResMut<NetherIdentity>>, http: Option<ResMut<NetherHttpServer>>) {
    let (Some(mut identity), Some(mut http)) = (identity, http) else { return };
    match identity.0.renew(IDENTITY_RENEWAL) {
        Some(Ok(renewed)) => http.set_identity(renewed),
        Some(Err(error)) => {
            let _tag = tracing::info_span!("network").entered();
            error!("failed to renew nethernet identity: {error}");
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chorus_core::config::Config;

    #[test]
    fn the_signaler_defaults_to_http_and_is_read_from_the_plugin_section() {
        assert_eq!(NetherNetConfig::default().signaler, Signaler::Http);
        let config = Config::from_table(
            toml::from_str(
                "[nethernet]
signaler = \"lan\"",
            )
            .unwrap(),
        );
        let section = config.get::<NetherNetConfig>();

        assert!(section.signaler.lan() && !section.signaler.http());
    }
}
