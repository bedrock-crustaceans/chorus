use crate::entity::entity::Entity as PlayerEntity;
use crate::level::level::Level;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::player::Player;
use crate::server::ServerState;
use std::sync::Arc;
use bedrock::protocol::v662::enums::{
    ChatRestrictionLevel, Difficulty, EditorWorldType, EducationEditionOffer, GamePublishSetting, GameType, GeneratorType, PlayStatus, PlayerPermissionLevel, SpawnBiomeType,
};
use bedrock::protocol::v662::packets::{LevelChunkPacket, NetworkChunkPublisherUpdatePacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID, BaseGameVersion, BlockPos, ChunkPos, EduSharedUriResource, Experiments, NetworkPermissions, SpawnSettings};
use bedrock::protocol::v818::types::SyncedPlayerMovementSettings;
use bedrock::protocol::v924::types::{GameRuleLegacyData, LevelSettings};
use bedrock::protocol::v944::packets::{StartGamePacket, VoxelShapesPacket};
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bedrock::protocol::{ProtoVersion, ProtoVersionPackets};
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{Commands, Query};
use bevy_ecs::system::ResMut;
use tracing::{debug, warn};

pub fn on_enter_setup(mut sessions: Query<&mut Session>, mut server_state: ResMut<ServerState>, mut state_reader: MessageReader<SessionStateChangedMessage>, mut commands: Commands) {
    for ev in state_reader.read() {
        if ev.to != SessionState::Setup {
            continue;
        }

        let Ok(mut session) = sessions.get_mut(ev.entity) else {
            continue;
        };

        let player = Player::new(server_state.get_runtime_id());

        session.send_immediate(BedrockProtocol::VoxelShapesPacket(
            VoxelShapesPacket {
                shapes: vec![],
                names: vec![],
                custom_shape_count: 0,
            }
            .into(),
        ));

        send_start_game(&player, &session);

        // TODO: Level should come from a shared Bevy Resource once it holds state
        let entity = PlayerEntity::default(
            "minecraft:player".to_string(),
            player.unique_id(),
            Arc::new(Level {}),
        );
        commands.entity(ev.entity).insert((player, entity));

        session.send_play_status(PlayStatus::PlayerSpawn, false);
    }
}

fn send_start_game(player: &Player, session: &Session) {
    session.send_immediate(BedrockProtocol::StartGamePacket(
        StartGamePacket {
            target_actor_id: ActorUniqueID(player.unique_id()),
            target_runtime_id: ActorRuntimeID(player.runtime_id()),
            actor_game_type: GameType::Survival,
            position: Default::default(),
            rotation: Default::default(),
            settings: LevelSettings {
                seed: 0,
                spawn_settings: SpawnSettings {
                    spawn_type: SpawnBiomeType::Default,
                    user_defined_biome_name: "plains".to_string(),
                    dimension: 0,
                },
                generator_type: GeneratorType::Flat,
                game_type: GameType::Survival,
                is_hardcore_enabled: false,
                game_difficulty: Difficulty::Peaceful,
                default_spawn_block_position: NetworkBlockPosition { x: 0, y: 0, z: 0 },
                achievements_disabled: false,
                editor_world_type: EditorWorldType::NonEditor,
                is_created_in_editor: false,
                is_exported_from_editor: false,
                day_cycle_stop_time: 0,
                education_edition_offer: EducationEditionOffer::None,
                education_features_enabled: false,
                education_product_id: "".to_string(),
                rain_level: 0.0,
                lightning_level: 0.0,
                has_confirmed_platform_locked_content: false,
                multiplayer_enabled: false,
                lan_broadcasting_enabled: false,
                xbox_live_broadcast_setting: GamePublishSetting::Public,
                platform_broadcast_setting: GamePublishSetting::Public,
                commands_enabled: false,
                texture_packs_required: false,
                rule_data: GameRuleLegacyData { rules_list: vec![] },
                experiments: Experiments {
                    experiments: vec![],
                    ever_toggled: false,
                },
                bonus_chest_enabled: false,
                starting_map_enabled: false,
                player_permissions: PlayerPermissionLevel::Member,
                server_chunk_tick_range: 0,
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
                base_game_version: BaseGameVersion("*".to_string()),
                limited_world_width: 0,
                limited_world_depth: 0,
                nether_type: false,
                edu_shared_uri_resource: EduSharedUriResource {
                    button_name: "".to_string(),
                    link_uri: "".to_string(),
                },
                override_force_experimental_gameplay: None,
                chat_restriction_level: ChatRestrictionLevel::None,
                disable_player_interactions: false,
            },
            level_id: "".to_string(),
            level_name: "".to_string(),
            template_content_identity: "".to_string(),
            is_trial: false,
            movement_settings: SyncedPlayerMovementSettings {
                rewind_history_size: 0,
                server_authoritative_block_breaking: false,
            },
            current_level_time: 0,
            enchantment_seed: 0,
            block_properties: vec![],
            multiplayer_correlation_id: "".to_string(),
            enable_item_stack_net_manager: false,
            server_version: BedrockProtocol::GAME_VERSION.to_string(),
            player_property_data: Default::default(),
            server_block_type_registry_checksum: 0,
            world_template_id: Default::default(),
            server_enabled_client_side_generation: false,
            block_network_ids_are_hashes: false,
            network_permissions: NetworkPermissions { server_auth_sound_enabled: false },
            server_join_information: None,
            server_id: "".to_string(),
            world_id: "".to_string(),
            scenario_id: "".to_string(),
            owner_id: "".to_string(),
        }
        .into(),
    ))
}

pub fn handle_setup(mut packet_reader: MessageReader<PacketReceivedMessage>, mut state_writer: MessageWriter<SessionStateChangedMessage>, mut query: Query<(&Player, &mut Session)>) {
    for ev in packet_reader.read() {
        let Ok(mut query) = query.get_mut(ev.entity) else {
            continue;
        };

        if query.1.get_state() != SessionState::Setup {
            continue;
        }

        match &ev.packet {
            BedrockProtocol::RequestChunkRadiusPacket(packet) => handle_request_chunk_radius(packet, &mut query.1),
            BedrockProtocol::SetLocalPlayerAsInitializedPacket(packet) => handle_set_local_player_as_initialized(packet, query.0, &mut query.1, &mut state_writer),
            packet => {
                warn!("unexpected packet received in setup state: {:?}", packet)
            }
        }
    }
}

fn handle_request_chunk_radius(packet: &<BedrockProtocol as ProtoVersionPackets>::RequestChunkRadiusPacket, session: &mut Session) {
    let radius = packet.chunk_radius;

    session.send(BedrockProtocol::NetworkChunkPublisherUpdatePacket(
        NetworkChunkPublisherUpdatePacket {
            new_view_position: BlockPos { x: 0, y: 0, z: 0 },
            new_view_radius: (radius << 4) as u32,
            server_built_chunks: vec![],
        }
        .into(),
    ));

    for x in -radius..radius {
        for z in -radius..radius {
            session.send(BedrockProtocol::LevelChunkPacket(
                LevelChunkPacket {
                    chunk_position: ChunkPos { x, z },
                    dimension_id: 0,
                    sub_chunk_count: 0,
                    sub_chunk_limit: 0,
                    cache_enabled: false,
                    cache_blobs: vec![],
                    serialized_chunk_data: vec![],
                }
                .into(),
            ))
        }
    }
    debug!("received {:?}", packet);
}

fn handle_set_local_player_as_initialized(
    packet: &<BedrockProtocol as ProtoVersionPackets>::SetLocalPlayerAsInitializedPacket,
    player: &Player,
    session: &mut Session,
    state_writer: &mut MessageWriter<SessionStateChangedMessage>,
) {
    if packet.player_id.0 != player.runtime_id() {
        warn!("received unexpected player_id {}, expected {}", packet.player_id.0, player.runtime_id());
        return;
    };

    session.set_state(SessionState::Play, state_writer);
}
