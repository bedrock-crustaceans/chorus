use crate::config::Config;
use crate::item::ItemTakenMessage;
use crate::level::{BlockDestroyedMessage, BlockUpdatedMessage};
use crate::network::BedrockProtocol;
use crate::network::handler::chat::PlayerChatMessage;
use crate::network::handler::{PacketHandlers, PacketReceivedMessage};
use crate::network::login::auth::LoginAuthOIDC;
use crate::network::session::Session;
use crate::network::session::state::SessionStateChangedMessage;
use bedrock::network::connection::Connection;
use bedrock::network::listener::Listener;
use bedrock::protocol::{ProtoVersion, Unknown};
use bevy_app::{App, FixedUpdate, Plugin, Startup};
use bevy_ecs::prelude::*;
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
            .add_systems(FixedUpdate, Network::tick)
            .add_message::<PacketReceivedMessage>()
            .add_message::<SessionStateChangedMessage>()
            .add_message::<BlockUpdatedMessage>()
            .add_message::<BlockDestroyedMessage>()
            .add_message::<ItemTakenMessage>()
            .add_message::<PlayerChatMessage>();
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

    pub fn tick(network: Res<NetworkState>, mut query: Query<(Entity, &mut Session)>, mut events: MessageWriter<PacketReceivedMessage>, mut commands: Commands) {
        for conn in network.incoming.try_iter() {
            let mut entity = commands.spawn_empty();
            entity.insert(Session::new(entity.id(), conn, &network.runtime));
        }

        for (entity, mut session) in query.iter_mut() {
            session.tick();

            while let Some(packet) = session.recv() {
                events.write(PacketReceivedMessage { entity, packet });
            }

            if session.is_closed() {
                commands.entity(entity).despawn();
            }
        }
    }
}
