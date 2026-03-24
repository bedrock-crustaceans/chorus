use bedrockrs::core::world::dimension::Dimension;
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
use bedrockrs::proto::v818::packets::ResourcePacksInfoPacket;
use bedrockrs::proto::v818::types::SyncedPlayerMovementSettings;
use bedrockrs::proto::v898::packets::ResourcePackStackPacket;
use bedrockrs::proto::v924::types::{GameRuleLegacyData, LevelSettings};
use bedrockrs::proto::v944::packets::{StartGamePacket, VoxelShapesPacket};
use bedrockrs::proto::v944::types::NetworkBlockPosition;
use bedrockrs::proto::{ProtoVersion, Unknown, V944};
use log::debug;
use std::collections::HashMap;
use uuid::Uuid;

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
                if req.client_network_version == 944 =>
            {
                unknown_conn.into_ver::<V944>()
            }
            _ => {
                debug!("Unsupported protocol");
                return;
            }
        };

        debug!("RequestNetworkSettings");

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
            }),
            V944::ResourcePacksInfoPacket(ResourcePacksInfoPacket {
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

        debug!("LoginSuccess + Resource packs sent");

        let _ = conn.recv().await; // ClientCacheStatus
        let _ = conn.recv().await; // ResourcePackClientResponse

        debug!("Client resource flow done");

        conn.send(&[V944::VoxelShapesPacket(VoxelShapesPacket {
            shapes: vec![],
            names: vec![],
            custom_shape_count: 0,
        })])
        .await
        .unwrap();

        debug!("Voxel shapes sent");

        conn.send(&[
            V944::StartGamePacket(StartGamePacket {
                target_actor_id: ActorUniqueID(609),
                target_runtime_id: ActorRuntimeID(402),
                actor_game_type: GameType::Creative,
                position: Default::default(),
                rotation: Default::default(),
                settings: LevelSettings {
                    seed: 777777777777,
                    spawn_settings: SpawnSettings {
                        spawn_type: SpawnBiomeType::Default,
                        user_defined_biome_name: String::from("RandomBiome"),
                        dimension: i32::from(Dimension::Overworld),
                    },
                    generator_type: GeneratorType::Overworld,
                    game_type: GameType::Creative,
                    is_hardcore_enabled: false,
                    game_difficulty: Difficulty::Peaceful,
                    default_spawn_block_position: NetworkBlockPosition {
                        x: 100,
                        y: 200,
                        z: 300,
                    },
                    achievements_disabled: true,
                    editor_world_type: EditorWorldType::NonEditor,
                    is_created_in_editor: false,
                    is_exported_from_editor: false,
                    day_cycle_stop_time: 2000,
                    education_edition_offer: EducationEditionOffer::None,
                    education_features_enabled: false,
                    education_product_id: String::from(""),
                    rain_level: 300.0,
                    lightning_level: 400.0,
                    has_confirmed_platform_locked_content: false,
                    multiplayer_enabled: true,
                    lan_broadcasting_enabled: true,
                    xbox_live_broadcast_setting: GamePublishSetting::FriendsOnly,
                    platform_broadcast_setting: GamePublishSetting::FriendsOnly,
                    commands_enabled: true,
                    texture_packs_required: false,
                    rule_data: GameRuleLegacyData { rules_list: vec![] },
                    experiments: Experiments {
                        experiments: vec![],
                        ever_toggled: false,
                    },
                    bonus_chest_enabled: false,
                    starting_map_enabled: false,
                    player_permissions: PlayerPermissionLevel::Custom,
                    server_chunk_tick_range: 4,
                    locked_behaviour_pack: false,
                    locked_resource_pack: false,
                    from_locked_template: false,
                    use_msa_gamer_tags: false,
                    from_template: false,
                    has_locked_template_settings: false,
                    only_spawn_v1_villagers: false,
                    persona_disabled: false,
                    custom_skins_disabled: false,
                    emote_chat_muted: false,
                    base_game_version: BaseGameVersion(V944::GAME_VERSION.to_string()),
                    limited_world_width: 16,
                    limited_world_depth: 16,
                    nether_type: true,
                    edu_shared_uri_resource: EduSharedUriResource {
                        button_name: String::from(""),
                        link_uri: String::from(""),
                    },
                    override_force_experimental_gameplay: Some(true),
                    chat_restriction_level: ChatRestrictionLevel::None,
                    disable_player_interactions: false,
                },
                level_id: String::from("UmFuZG9tIFdvcmxk"),
                level_name: String::from("Random World"),
                template_content_identity: String::from(""),
                is_trial: false,
                movement_settings: SyncedPlayerMovementSettings {
                    rewind_history_size: 3200,
                    server_authoritative_block_breaking: false,
                },
                current_level_time: 9000,
                enchantment_seed: 99000,
                block_properties: vec![],
                multiplayer_correlation_id: String::from("c5d3d2cc-27fd-4221-9de6-d22c4d423d53"),
                enable_item_stack_net_manager: false,
                server_version: V944::GAME_VERSION.to_string(),
                player_property_data: HashMap::new(),
                server_block_type_registry_checksum: 0,
                world_template_id: Uuid::nil(),
                server_enabled_client_side_generation: false,
                block_network_ids_are_hashes: false,
                network_permissions: NetworkPermissions {
                    server_auth_sound_enabled: false,
                },
                server_join_information: None,
                server_id: "".to_string(),
                world_id: "".to_string(),
                scenario_id: "".to_string(),
                owner_id: "".to_string(),
            }),
            V944::PlayStatusPacket(PlayStatusPacket {
                status: PlayStatus::LoginSuccess,
            }),
        ])
        .await
        .unwrap();

        debug!("StartGame and PlayStatus sent");

        loop {
            let res = conn.recv().await;

            if let Ok(packet) = res {
                debug!("Found packet: {:?}", packet);
            } else {
                break;
            }
        }
    }
}
