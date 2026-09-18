use crate::command::dispatch::{CommandPreprocessMessage, CommandRequestedMessage};
use crate::config::Config;
use crate::level::{BlockUpdatedMessage, LevelEventMessage, LevelSoundMessage};
use crate::network::BedrockProtocol;
use crate::network::bandwidth::BandwidthTracker;
use crate::network::handler::block::{BlockBreakMessage, BlockInteractMessage, BlockPlaceMessage, PlayerDropItemMessage};
use crate::network::handler::chat::{BroadcastMessage, PlayerChatMessage};
use crate::network::handler::chunks::ChunkSentMessage;
use crate::network::handler::form::FormResponseMessage;
use crate::network::handler::inventory::{InventoryCloseMessage, InventoryOpenMessage, PlayerItemHeldMessage};
use crate::network::handler::login::PlayerLoginMessage;
use crate::network::handler::play::{
    PlayerItemUseMessage, PlayerJoinedMessage, PlayerJumpMessage, PlayerMoveMessage, PlayerQuitMessage, PlayerToggleFlightMessage, PlayerToggleSneakMessage, PlayerToggleSprintMessage,
};
use crate::network::handler::request::PlayerPreLoginMessage;
use crate::network::handler::resource::ResourcePackResponseMessage;
use crate::network::handler::setup::PlayerChunkRadiusMessage;
use crate::network::handler::{PacketHandlers, PacketReceivedMessage};
use crate::network::login::auth::LoginAuthOIDC;
use crate::network::session::state::SessionStateChangedMessage;
use crate::network::session::{PlayerDisconnectMessage, PlayerKickedMessage, Session, detect_session_close};
use bedrock::network::connection::Connection;
use bedrock::network::listener::Listener;
use bedrock::protocol::{ProtoVersion, Unknown};
use bevy_app::{App, Last, Plugin, PostUpdate, PreUpdate, Startup};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::IntoScheduleConfigs;
use crossbeam_channel::Receiver;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use tokio::task::JoinHandle;
use tracing::{error, info};

#[derive(Resource)]
pub struct NetworkState {
    incoming: Receiver<Connection<Unknown>>,
    runtime: tokio::runtime::Runtime,
    listener_task: JoinHandle<()>,
}

pub struct Network;

impl Plugin for Network {
    fn build(&self, app: &mut App) {
        app.add_plugins(PacketHandlers)
            .add_plugins(LoginAuthOIDC)
            .add_systems(Startup, Network::init)
            .add_systems(PreUpdate, Network::receive)
            .add_systems(PostUpdate, detect_session_close.before(Network::flush))
            .add_systems(PostUpdate, Network::flush)
            .add_systems(Last, BandwidthTracker::sample)
            .init_resource::<BandwidthTracker>()
            .add_message::<PacketReceivedMessage>()
            .add_message::<SessionStateChangedMessage>()
            .add_message::<PlayerJoinedMessage>()
            .add_message::<PlayerQuitMessage>()
            .add_message::<PlayerKickedMessage>()
            .add_message::<PlayerDisconnectMessage>()
            .add_message::<PlayerLoginMessage>()
            .add_message::<PlayerPreLoginMessage>()
            .add_message::<PlayerChunkRadiusMessage>()
            .add_message::<PlayerMoveMessage>()
            .add_message::<PlayerToggleSneakMessage>()
            .add_message::<PlayerToggleSprintMessage>()
            .add_message::<PlayerToggleFlightMessage>()
            .add_message::<PlayerJumpMessage>()
            .add_message::<PlayerItemUseMessage>()
            .add_message::<ResourcePackResponseMessage>()
            .add_message::<BlockUpdatedMessage>()
            .add_message::<BlockBreakMessage>()
            .add_message::<BlockPlaceMessage>()
            .add_message::<PlayerDropItemMessage>()
            .add_message::<BlockInteractMessage>()
            .add_message::<InventoryOpenMessage>()
            .add_message::<InventoryCloseMessage>()
            .add_message::<PlayerItemHeldMessage>()
            .add_message::<FormResponseMessage>()
            .add_message::<ChunkSentMessage>()
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
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(config.threads).enable_all().build().unwrap();

        let mut listener = runtime.block_on(async {
            let mut listener = Listener::new_raknet(
                SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::from_str(config.ip.as_str()).unwrap_or_else(|err| {
                        error!("{}: {}", err, config.ip);

                        Ipv4Addr::UNSPECIFIED
                    })),
                    config.port,
                ),
                config.name.clone(),
                config.sub_name.clone(),
                String::from(BedrockProtocol::GAME_VERSION),
                BedrockProtocol::PROTOCOL_VERSION,
                BedrockProtocol::RAKNET_VERSION,
                config.max_players,
                0,
                false,
            )
            .await
            .unwrap();

            listener.start().await.unwrap();
            listener
        });

        let (incoming_send, incoming_recv) = crossbeam_channel::unbounded();

        let listener_task = runtime.spawn(async move {
            loop {
                let conn = listener.accept().await.unwrap();

                info!("Connected: {}", conn.get_socket_addr().ip().to_string());

                incoming_send.send(conn).unwrap();
            }
        });

        commands.insert_resource(NetworkState {
            incoming: incoming_recv,
            runtime,
            listener_task,
        })
    }

    /// Accepts new connections and drains everything the connection tasks received since the last
    /// tick. Runs before the packet handlers so that a packet never waits a whole tick to be seen.
    pub fn receive(network: Res<NetworkState>, bandwidth: Res<BandwidthTracker>, mut query: Query<(Entity, &mut Session)>, mut events: MessageWriter<PacketReceivedMessage>, mut commands: Commands) {
        for conn in network.incoming.try_iter() {
            let mut entity = commands.spawn_empty();
            entity.insert(Session::new(entity.id(), conn, &network.runtime, bandwidth.counters()));
        }

        for (entity, mut session) in query.iter_mut() {
            while let Some(packet) = session.recv() {
                events.write(PacketReceivedMessage { entity, packet });
            }
        }
    }

    /// Pushes everything the handlers queued this tick out, and reaps closed sessions.
    pub fn flush(mut query: Query<(Entity, &mut Session)>, mut commands: Commands) {
        for (entity, mut session) in query.iter_mut() {
            session.flush();

            if session.is_closed() {
                commands.entity(entity).despawn();
            }
        }
    }
}
