use crate::command::registry::CommandRegistry;
use crate::command::sender::CommandSender;
use crate::entity::entity::Entity as PlayerEntity;
use crate::level::BlockUpdatedMessage;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::handler::chat::{PlayerChatMessage, handle_text};
use crate::network::handler::form::handle_modal_form_response;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::player::Player;
use crate::player::identity::PlayerIdentity;
use bedrock::protocol::v662::enums::{ActorFlags, CommandPermissionLevel, DataItemType};
use bedrock::protocol::v662::packets::{SetActorDataPacket, UpdateAbilitiesPacket, UpdateBlockPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, DataItem, PropertySyncData};
use bedrock::protocol::v729::packets::{AttributeData, UpdateAttributesPacket};
use bedrock::protocol::v776::enums::AbilitiesIndex;
use bedrock::protocol::v776::types::{SerializedAbilitiesData, SerializedAbilitiesLayer, SerializedLayer};
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{Query, Res};
use tracing::{debug, warn};
use vek::{Vec2, Vec3};

pub fn on_enter_play(mut sessions: Query<(&mut Session, &Player)>, commands: Res<CommandRegistry>, mut state_reader: MessageReader<SessionStateChangedMessage>) {
    for ev in state_reader.read() {
        if ev.to != SessionState::Play {
            continue;
        }
        let Ok((mut session, player)) = sessions.get_mut(ev.entity) else {
            continue;
        };

        debug!("on_enter_play");

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

pub fn handle_play(
    mut packet_reader: MessageReader<PacketReceivedMessage>,
    mut chat_writer: MessageWriter<PlayerChatMessage>,
    commands: Res<CommandRegistry>,
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
                entity.position = Vec3::new(x, y, z);
                let (pitch, yaw) = packet.player_rotation;
                entity.rotation = Vec2::new(pitch, yaw);
            }
            BedrockProtocol::TextPacket(packet) => handle_text(packet, identity, &mut chat_writer),
            BedrockProtocol::CommandRequestPacket(packet) => {
                let mut sender = CommandSender::new(&mut session, identity.name().to_string(), Some(&mut player));
                commands.dispatch(&packet.command, &mut sender);
            }
            BedrockProtocol::ModalFormResponsePacket(packet) => handle_modal_form_response(packet, &mut player),
            packet => {
                warn!("unexpected packet received in play state: {:?}", packet);
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
