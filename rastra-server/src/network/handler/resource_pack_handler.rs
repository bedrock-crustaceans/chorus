use bedrockrs::network::connection::Connection;
use bedrockrs::proto::v662::types::{BaseGameVersion, Experiments};
use bedrockrs::proto::v818::packets::ResourcePacksInfoPacket;
use bedrockrs::proto::v898::packets::ResourcePackStackPacket;
use bedrockrs::proto::V944;
use log::debug;
use uuid::Uuid;
use crate::network::handler::game_start_handler::GameStartHandler;

pub struct ResourcePackHandler;

impl ResourcePackHandler {
    
    pub async fn handle(mut conn: Connection<V944>) {
        conn.send(&[V944::ResourcePacksInfoPacket(ResourcePacksInfoPacket {
            resource_pack_required: false,
            has_addon_packs: false,
            has_scripts: false,
            force_disable_vibrant_visuals: false,
            world_template_uuid: Uuid::nil(),
            resource_packs: vec![],
            world_template_version: "".to_string(),
        }),
            V944::ResourcePackStackPacket(ResourcePackStackPacket {
                texture_pack_required: false,
                addon_list: vec![],
                base_game_version: BaseGameVersion("1.0".to_string()),
                experiments: Experiments {
                    experiments: vec![],
                    ever_toggled: false,
                },
                include_editor_packs: false,
            }),
        ])
            .await
            .unwrap();

        let _ = conn.recv().await; // ClientCacheStatus
        let _ = conn.recv().await; // ResourcePackClientResponse

        debug!("Client resource flow done");
        
        GameStartHandler::handle(conn).await;
    }
}