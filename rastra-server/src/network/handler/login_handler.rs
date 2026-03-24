use bedrockrs::network::connection::Connection;
use bedrockrs::proto::compression::Compression;
use bedrockrs::proto::v662::enums::{
    ChatRestrictionLevel, Difficulty, EditorWorldType, EducationEditionOffer, GamePublishSetting,
    GameType, GeneratorType, PacketCompressionAlgorithm, PlayStatus, PlayerPermissionLevel,
    SpawnBiomeType,
};
use bedrockrs::proto::v662::packets::{NetworkSettingsPacket, PlayStatusPacket};
use bedrockrs::proto::v662::types::{
    ActorRuntimeID, ActorUniqueID, BaseGameVersion, EduSharedUriResource, Experiments,
    NetworkPermissions, SpawnSettings,
};
use bedrockrs::proto::{ProtoVersion, Unknown, V944};
use log::debug;
use crate::network::handler::resource_pack_handler::ResourcePackHandler;

pub struct LoginHandler;

impl LoginHandler {
    pub async fn handle(mut unknown_conn: Connection<Unknown>) {
        debug!("Got connection from {:?}", unknown_conn.get_socket_addr());

        let packets = match unknown_conn.recv().await {
            Ok(p) => p,
            Err(e) => {
                debug!("Connection closed early: {:?}", e);
                return;
            }
        };

        let mut conn = match packets.first() {
            Some(Unknown::RequestNetworkSettingsPacket(req))
            if req.client_network_version as u32 == V944::PROTOCOL_VERSION =>
                {
                    unknown_conn.into_ver::<V944>()
                }
            _ => {
                debug!("Unsupported protocol");
                return;
            }
        };

        debug!("RequestNetworkSettings received");

        conn.send(&[V944::NetworkSettingsPacket(NetworkSettingsPacket {
            compression_threshold: 1,
            compression_algorithm: PacketCompressionAlgorithm::None,
            client_throttle_enabled: false,
            client_throttle_threshold: 0,
            client_throttle_scalar: 0.0,
        })])
        .await
        .unwrap();

        conn.compression = Some(Compression::None);

        debug!("NetworkSettings sent");

        if conn.recv().await.is_err() {
            debug!("Login packet failed");
            return;
        }

        debug!("Login received");

        conn.send(&[
            V944::PlayStatusPacket(PlayStatusPacket {
                status: PlayStatus::LoginSuccess,
            })])
        .await
        .unwrap();

        debug!("PlayStatus sent");

        ResourcePackHandler::handle(conn).await;
    }
}
