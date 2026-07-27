use crate::level::level::Level;
use crate::level::{BlockDestroyedMessage, BlockUpdatedMessage};
use crate::math::enums::block_face::BlockFace;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::handler::item::spawn_block_drop;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::registry::block_registry::BlockRegistry;
use crate::server::ServerState;
use bedrock::protocol::v662::enums::{ItemUseInventoryTransactionType, LevelEvent, PlayerActionType};
use bedrock::protocol::v662::packets::LevelEventPacket;
use bevy_ecs::change_detection::ResMut;
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{Commands, Query, Res};

const OVERWORLD: i32 = 0;

pub fn handle_block_interaction(
    mut reader: MessageReader<PacketReceivedMessage>,
    query: Query<&Session>,
    mut level: ResMut<Level>,
    registry: Res<BlockRegistry>,
    mut commands: Commands,
    mut server_state: ResMut<ServerState>,
    mut block_writer: MessageWriter<BlockUpdatedMessage>,
    mut destroy_writer: MessageWriter<BlockDestroyedMessage>,
) {
    let air_id = registry.get_block_id("minecraft:air").unwrap_or(0);

    for ev in reader.read() {
        let BedrockProtocol::PlayerAuthInputPacket(packet) = &ev.packet else {
            continue;
        };
        let Ok(session) = query.get(ev.entity) else {
            continue;
        };
        if session.get_state() != SessionState::Play {
            continue;
        }

        if let Some(actions) = &packet.player_block_actions {
            for action in actions {
                if !matches!(action.action_type, PlayerActionType::PredictDestroyBlock) {
                    continue;
                }
                let Some(pos) = &action.position else {
                    continue;
                };
                break_and_drop(&mut level, &mut commands, &mut server_state, pos.x, pos.y, pos.z, air_id, &mut block_writer, &mut destroy_writer);
            }
        }

        if let Some(transaction) = &packet.item_use_transaction {
            let pos = &transaction.position;
            match transaction.action_type {
                ItemUseInventoryTransactionType::Destroy => {
                    break_and_drop(&mut level, &mut commands, &mut server_state, pos.x, pos.y, pos.z, air_id, &mut block_writer, &mut destroy_writer);
                }
                ItemUseInventoryTransactionType::Place => {
                    if let Some(block_id) = transaction.item.block_runtime_id
                        && block_id != 0
                        && block_id != air_id
                        && let Ok(face) = BlockFace::from_index(transaction.face as usize)
                    {
                        let x = pos.x + face.get_x_offset() as i32;
                        let y = pos.y + face.get_y_offset() as i32;
                        let z = pos.z + face.get_z_offset() as i32;
                        level.place_block(OVERWORLD, x, y, z, block_id, air_id, &mut block_writer);
                    }
                }
                ItemUseInventoryTransactionType::Use => {}
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn break_and_drop(
    level: &mut Level,
    commands: &mut Commands,
    server_state: &mut ServerState,
    x: i32,
    y: i32,
    z: i32,
    air_id: i32,
    block_writer: &mut MessageWriter<BlockUpdatedMessage>,
    destroy_writer: &mut MessageWriter<BlockDestroyedMessage>,
) {
    let Some(previous) = level.break_block(OVERWORLD, x, y, z, air_id, block_writer) else {
        return;
    };
    destroy_writer.write(BlockDestroyedMessage {
        dimension_id: OVERWORLD,
        x,
        y,
        z,
        block_id: previous,
    });
    spawn_block_drop(commands, server_state, previous, x, y, z);
}

pub fn broadcast_block_destroy(mut reader: MessageReader<BlockDestroyedMessage>, mut query: Query<&mut Session>) {
    for msg in reader.read() {
        for mut session in &mut query {
            session.send(BedrockProtocol::LevelEventPacket(
                LevelEventPacket {
                    event_id: LevelEvent::ParticlesDestroyBlock as i32,
                    position: (msg.x as f32, msg.y as f32, msg.z as f32),
                    data: msg.block_id,
                }
                .into(),
            ));
        }
    }
}
