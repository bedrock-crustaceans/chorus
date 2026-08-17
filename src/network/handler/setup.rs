use crate::entity::entity::Entity as PlayerEntity;
use crate::level::DimensionId;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::player::Player;
use crate::registry::item_registry::ItemRegistry;
use crate::server::ServerState;
use bedrock::protocol::v662::enums::{ChatRestrictionLevel, Difficulty, EditorWorldType, GamePublishSetting, GameType, GeneratorType, PlayStatus, PlayerPermissionLevel, SpawnBiomeType};
use bedrock::protocol::v662::packets::ChunkRadiusUpdatedPacket;
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID, BaseGameVersion, EduSharedUriResource, Experiments, NetworkPermissions, SpawnSettings};
use bedrock::protocol::v800::packets::BiomeDefinitionListPacket;
use bedrock::protocol::v818::types::SyncedPlayerMovementSettings;
use bedrock::protocol::v944::packets::VoxelShapesPacket;
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bedrock::protocol::v2168::enums::EducationEditionOffer;
use bedrock::protocol::v2168::packets::StartGamePacket;
use bedrock::protocol::v2168::types::{GameRuleLegacyData, LevelSettings};
use bedrock::protocol::{ProtoVersion, ProtoVersionPackets};
use bevy_ecs::message::{Message, MessageReader, MessageWriter};
use bevy_ecs::prelude::{Commands, Entity, Query};
use bevy_ecs::system::{Res, ResMut};
use tracing::{debug, warn};

pub fn on_enter_setup(
    mut sessions: Query<&mut Session>,
    mut server_state: ResMut<ServerState>,
    items: Res<ItemRegistry>,
    mut state_reader: MessageReader<SessionStateChangedMessage>,
    mut commands: Commands,
) {
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

        send_start_game(&player, &mut session);

        session.send_immediate(BedrockProtocol::ItemComponentPacket(items.to_packet().into()));

        let entity = PlayerEntity::default("minecraft:player".to_string(), player.unique_id());
        commands.entity(ev.entity).insert((player, entity, DimensionId(0)));
    }
}

fn send_start_game(player: &Player, session: &mut Session) {
    session.send_immediate(BedrockProtocol::StartGamePacket(
        StartGamePacket {
            target_actor_id: ActorUniqueID(player.unique_id()),
            target_runtime_id: ActorRuntimeID(player.runtime_id()),
            actor_game_type: player.gamemode().game_type(),
            position: (0.5, 6.0, 0.5), // TODO: those shouldn't be hardcoded, maybe player db?
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
                default_spawn_block_position: NetworkBlockPosition { x: 0, y: 4, z: 0 },
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
                commands_enabled: true,
                texture_packs_required: false,
                rule_data: GameRuleLegacyData { rules_list: vec![] },
                experiments: Experiments {
                    experiments: vec![],
                    ever_toggled: false,
                },
                bonus_chest_enabled: false,
                starting_map_enabled: false,
                player_permissions: PlayerPermissionLevel::Member as u8,
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
                server_editor_connection_policy: 0,
                allow_anonymous_block_drops_in_editor_worlds: false,
            },
            level_id: "".to_string(),
            level_name: "".to_string(),
            template_content_identity: "".to_string(),
            is_trial: false,
            movement_settings: SyncedPlayerMovementSettings {
                rewind_history_size: 0,
                server_authoritative_block_breaking: true,
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
            block_network_ids_are_hashes: true,
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

#[derive(Message, Clone, Debug)]
pub struct PlayerChunkRadiusMessage {
    pub entity: Entity,
    pub radius: i32,
}

pub fn handle_setup(
    mut packet_reader: MessageReader<PacketReceivedMessage>,
    items: Res<ItemRegistry>,
    mut state_writer: MessageWriter<SessionStateChangedMessage>,
    mut chunk_radius_writer: MessageWriter<PlayerChunkRadiusMessage>,
    mut query: Query<(&mut Player, &mut Session)>,
) {
    for ev in packet_reader.read() {
        let Ok((mut player, mut session)) = query.get_mut(ev.entity) else {
            continue;
        };
        if session.get_state() != SessionState::Setup {
            continue;
        }
        match &ev.packet {
            BedrockProtocol::RequestChunkRadiusPacket(packet) => handle_request_chunk_radius(ev.entity, packet, &mut player, &mut session, &items, &mut chunk_radius_writer),
            BedrockProtocol::SetLocalPlayerAsInitializedPacket(packet) => handle_set_local_player_as_initialized(packet, &player, &mut session, &mut state_writer),
            packet => {
                let count = session.unhandled_packets.entry(packet.as_ref().meta().name).or_insert(0);
                *count = count.saturating_add(1);
            }
        }
    }
}

fn handle_request_chunk_radius(
    entity: Entity,
    packet: &<BedrockProtocol as ProtoVersionPackets>::RequestChunkRadiusPacket,
    player: &mut Player,
    session: &mut Session,
    items: &ItemRegistry,
    chunk_radius_writer: &mut MessageWriter<PlayerChunkRadiusMessage>,
) {
    let radius = packet.chunk_radius.min(8);
    debug!("RequestChunkRadius: requested={}, capped={}", packet.chunk_radius, radius);

    // the queue itself is filled by update_chunk_order, which also keeps it following the player
    player.chunks_radius = radius;

    chunk_radius_writer.write(PlayerChunkRadiusMessage { entity, radius });

    session.send(BedrockProtocol::ChunkRadiusUpdatedPacket(ChunkRadiusUpdatedPacket { chunk_radius: radius }.into()));

    session.send(BedrockProtocol::BiomeDefinitionListPacket(BiomeDefinitionListPacket { biomes: vec![], strings: vec![] }.into()));

    session.send_play_status(PlayStatus::PlayerSpawn, false);

    session.send(BedrockProtocol::CreativeContentPacket(items.to_creative_packet().into()));
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
