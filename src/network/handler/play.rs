use crate::command::dispatch::{CommandPreprocessMessage, CommandRequestedMessage};
use crate::entity::entity::Entity as PlayerEntity;
use crate::item::item_stack::ItemStack;
use crate::level::BlockUpdatedMessage;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::handler::chat::{BroadcastMessage, PlayerChatMessage, handle_text};
use crate::network::handler::form::{FormResponseMessage, handle_modal_form_response};
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::player::Player;
use crate::player::identity::PlayerIdentity;
use crate::registry::command_registry::CommandRegistry;
use bedrock::protocol::v662::enums::{ActorFlags, CommandPermissionLevel};
use bedrock::protocol::v662::packets::{SetActorDataPacket, UpdateAbilitiesPacket, UpdateBlockPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, DataItem, PropertySyncData};
use bedrock::protocol::v729::packets::{AttributeData, UpdateAttributesPacket};
use bedrock::protocol::v776::enums::AbilitiesIndex;
use bedrock::protocol::v776::types::{SerializedAbilitiesData, SerializedAbilitiesLayer, SerializedLayer};
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bedrock::protocol::v2168::enums::{DataItemType, PlayerAuthInputData};
use bevy_ecs::message::{Message, MessageReader, MessageWriter};
use bevy_ecs::prelude::{Entity, Query, Res};
use glam::{Vec2, Vec3};
use tracing::{debug, info};

#[derive(Message, Clone, Debug)]
pub struct PlayerJoinedMessage {
    pub entity: Entity,
    pub name: String,
}

#[derive(Message, Clone, Debug)]
pub struct PlayerQuitMessage {
    pub entity: Entity,
    pub name: String,
}

pub fn on_enter_play(
    mut sessions: Query<(&mut Session, &Player, &PlayerIdentity)>,
    commands: Res<CommandRegistry>,
    mut state_reader: MessageReader<SessionStateChangedMessage>,
    mut join_writer: MessageWriter<PlayerJoinedMessage>,
) {
    for ev in state_reader.read() {
        if ev.to != SessionState::Play {
            continue;
        }
        let Ok((mut session, player, identity)) = sessions.get_mut(ev.entity) else {
            continue;
        };

        debug!("on_enter_play");

        info!("{} joined the game", identity.name());

        join_writer.write(PlayerJoinedMessage {
            entity: ev.entity,
            name: identity.name().to_string(),
        });

        session.send(BedrockProtocol::AvailableCommandsPacket(commands.to_packet().into()));

        let flags = (1i64 << ActorFlags::HasGravity as i64) | (1i64 << ActorFlags::HasCollision as i64) | (1i64 << ActorFlags::Breathing as i64);

        session.send(BedrockProtocol::SetActorDataPacket(
            SetActorDataPacket {
                target_runtime_id: ActorRuntimeID(player.runtime_id()),
                actor_data: vec![DataItem {
                    data_item_id: 0,
                    data_item_type: DataItemType::Int64(flags),
                }],
                synced_properties: PropertySyncData {
                    int_entries_list: vec![],
                    float_entries_list: vec![],
                },
                tick: 0,
            }
            .into(),
        ));

        let ability_values = (1u32 << AbilitiesIndex::Build as u32)
            | (1u32 << AbilitiesIndex::Mine as u32)
            | (1u32 << AbilitiesIndex::DoorsAndSwitches as u32)
            | (1u32 << AbilitiesIndex::OpenContainers as u32)
            | (1u32 << AbilitiesIndex::AttackPlayers as u32)
            | (1u32 << AbilitiesIndex::AttackMobs as u32);

        session.send(BedrockProtocol::UpdateAbilitiesPacket(
            UpdateAbilitiesPacket {
                data: SerializedAbilitiesData {
                    target_player_raw_id: player.unique_id(),
                    player_permissions: 1,
                    command_permissions: CommandPermissionLevel::Any,
                    layers: vec![SerializedLayer {
                        serialized_layer: SerializedAbilitiesLayer::Base,
                        abilities_set: 0xFFFFF,
                        ability_values,
                        fly_speed: 0.05,
                        vertical_fly_speed: 1.0,
                        walk_speed: 0.1,
                    }],
                },
            }
            .into(),
        ));

        let attribute = |name: &str, min: f32, max: f32, value: f32| AttributeData {
            min_value: min,
            max_value: max,
            current_value: value,
            default_min: min,
            default_max: max,
            default_value: value,
            attribute_name: name.to_string(),
            attribute_modifiers: vec![],
        };

        session.send(BedrockProtocol::UpdateAttributesPacket(
            UpdateAttributesPacket {
                target_runtime_id: ActorRuntimeID(player.runtime_id()),
                attribute_list: vec![
                    attribute("minecraft:movement", 0.0, f32::MAX, 0.1),
                    attribute("minecraft:underwater_movement", 0.0, f32::MAX, 0.02),
                    attribute("minecraft:lava_movement", 0.0, f32::MAX, 0.02),
                    attribute("minecraft:health", 0.0, 20.0, 20.0),
                    attribute("minecraft:player.hunger", 0.0, 20.0, 20.0),
                    attribute("minecraft:player.saturation", 0.0, 20.0, 20.0),
                    attribute("minecraft:player.level", 0.0, 24791.0, 0.0),
                    attribute("minecraft:player.experience", 0.0, 1.0, 0.0),
                ],
                ticks_since_sim_started: 0,
            }
            .into(),
        ));
    }
}

pub fn on_quit(sessions: Query<(Entity, &Session, &PlayerIdentity)>, mut quit_writer: MessageWriter<PlayerQuitMessage>) {
    for (entity, session, identity) in &sessions {
        if !session.is_closed() || session.get_state() != SessionState::Play {
            continue;
        }

        info!("{} left the game", identity.name());

        quit_writer.write(PlayerQuitMessage {
            entity,
            name: identity.name().to_string(),
        });
    }
}

pub fn announce_join_quit(mut join_reader: MessageReader<PlayerJoinedMessage>, mut quit_reader: MessageReader<PlayerQuitMessage>, mut broadcast_writer: MessageWriter<BroadcastMessage>) {
    for ev in join_reader.read() {
        broadcast_writer.write(BroadcastMessage::translate("§e%multiplayer.player.joined", vec![ev.name.clone()]));
    }

    for ev in quit_reader.read() {
        broadcast_writer.write(BroadcastMessage::translate("§e%multiplayer.player.left", vec![ev.name.clone()]));
    }
}

#[derive(Message, Clone, Debug)]
pub struct PlayerMoveMessage {
    pub entity: Entity,
    pub from_position: Vec3,
    pub to_position: Vec3,
    pub from_rotation: Vec2,
    pub to_rotation: Vec2,
}

#[derive(Message, Clone, Debug)]
pub struct PlayerToggleSneakMessage {
    pub entity: Entity,
    pub sneaking: bool,
}

#[derive(Message, Clone, Debug)]
pub struct PlayerToggleSprintMessage {
    pub entity: Entity,
    pub sprinting: bool,
}

#[derive(Message, Clone, Debug)]
pub struct PlayerToggleFlightMessage {
    pub entity: Entity,
    pub flying: bool,
}

#[derive(Message, Clone, Debug)]
pub struct PlayerJumpMessage {
    pub entity: Entity,
}

#[derive(Message, Clone, Debug)]
pub struct PlayerItemUseMessage {
    pub entity: Entity,
    pub item: ItemStack,
}

pub fn handle_play(
    mut packet_reader: MessageReader<PacketReceivedMessage>,
    mut chat_writer: MessageWriter<PlayerChatMessage>,
    mut command_preprocess_writer: MessageWriter<CommandPreprocessMessage>,
    mut command_writer: MessageWriter<CommandRequestedMessage>,
    mut move_writer: MessageWriter<PlayerMoveMessage>,
    mut sneak_writer: MessageWriter<PlayerToggleSneakMessage>,
    mut sprint_writer: MessageWriter<PlayerToggleSprintMessage>,
    mut flight_writer: MessageWriter<PlayerToggleFlightMessage>,
    mut jump_writer: MessageWriter<PlayerJumpMessage>,
    mut item_use_writer: MessageWriter<PlayerItemUseMessage>,
    mut form_writer: MessageWriter<FormResponseMessage>,
    mut query: Query<(&mut PlayerEntity, &mut Player, &mut Session, &PlayerIdentity)>,
) {
    for ev in packet_reader.read() {
        let Ok((mut entity, mut player, mut session, identity)) = query.get_mut(ev.entity) else {
            continue;
        };

        if session.get_state() != SessionState::Play {
            continue;
        }

        match &ev.packet {
            BedrockProtocol::PlayerAuthInputPacket(packet) => {
                let (x, y, z) = packet.player_position;
                let new_position = Vec3::new(x, y, z);
                let (pitch, yaw) = packet.player_rotation;
                let new_rotation = Vec2::new(pitch, yaw);

                if new_position != entity.position || new_rotation != entity.rotation {
                    move_writer.write(PlayerMoveMessage {
                        entity: ev.entity,
                        from_position: entity.position,
                        to_position: new_position,
                        from_rotation: entity.rotation,
                        to_rotation: new_rotation,
                    });
                }

                entity.position = new_position;
                entity.rotation = new_rotation;

                if packet.input_data.contains(&PlayerAuthInputData::StartSneaking) {
                    sneak_writer.write(PlayerToggleSneakMessage { entity: ev.entity, sneaking: true });
                } else if packet.input_data.contains(&PlayerAuthInputData::StopSneaking) {
                    sneak_writer.write(PlayerToggleSneakMessage { entity: ev.entity, sneaking: false });
                }

                if packet.input_data.contains(&PlayerAuthInputData::StartSprinting) {
                    sprint_writer.write(PlayerToggleSprintMessage { entity: ev.entity, sprinting: true });
                } else if packet.input_data.contains(&PlayerAuthInputData::StopSprinting) {
                    sprint_writer.write(PlayerToggleSprintMessage { entity: ev.entity, sprinting: false });
                }

                if packet.input_data.contains(&PlayerAuthInputData::StartFlying) {
                    flight_writer.write(PlayerToggleFlightMessage { entity: ev.entity, flying: true });
                } else if packet.input_data.contains(&PlayerAuthInputData::StopFlying) {
                    flight_writer.write(PlayerToggleFlightMessage { entity: ev.entity, flying: false });
                }

                if packet.input_data.contains(&PlayerAuthInputData::StartJumping) {
                    jump_writer.write(PlayerJumpMessage { entity: ev.entity });
                }

                if packet.input_data.contains(&PlayerAuthInputData::StartUsingItem) {
                    item_use_writer.write(PlayerItemUseMessage {
                        entity: ev.entity,
                        item: *player.inventory.held_item(),
                    });
                }
            }
            BedrockProtocol::TextPacket(packet) => handle_text(ev.entity, packet, identity, &mut chat_writer),
            BedrockProtocol::CommandRequestPacket(packet) => {
                command_preprocess_writer.write(CommandPreprocessMessage {
                    entity: ev.entity,
                    line: packet.command.clone(),
                });

                command_writer.write(CommandRequestedMessage {
                    entity: ev.entity,
                    line: packet.command.clone(),
                });
            }
            BedrockProtocol::ModalFormResponsePacket(packet) => handle_modal_form_response(ev.entity, packet, &mut player, &mut form_writer),
            packet => {
                let count = session.unhandled_packets.entry(packet.as_ref().meta().name).or_insert(0);
                *count = count.saturating_add(1);
            }
        }
    }
}

pub fn broadcast_block_updates(mut reader: MessageReader<BlockUpdatedMessage>, mut query: Query<&mut Session>) {
    for msg in reader.read() {
        for mut session in &mut query {
            session.send(BedrockProtocol::UpdateBlockPacket(
                UpdateBlockPacket {
                    block_position: NetworkBlockPosition { x: msg.x, y: msg.y, z: msg.z },
                    block_runtime_id: msg.block_id as u32,
                    flags: 0xB,
                    layer: msg.layer as u32,
                }
                .into(),
            ));
        }
    }
}
