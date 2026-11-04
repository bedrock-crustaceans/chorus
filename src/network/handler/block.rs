use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::world::block::BlockActionMessage;
use bedrock::protocol::v662::enums::{ItemUseInventoryTransactionType, PlayerActionType};
use bedrock::protocol::v662::types::InventoryTransactionData;
use bevy_ecs::message::{MessageReader, MessageWriter};
use glam::IVec3;

pub fn decode_block_actions(mut packet_reader: MessageReader<PacketReceivedMessage>, mut action_writer: MessageWriter<BlockActionMessage>) {
    for ev in packet_reader.read() {
        match &ev.packet {
            BedrockProtocol::PlayerActionPacket(packet) if !matches!(packet.action, PlayerActionType::StartItemUseOn) => {
                action_writer.write(BlockActionMessage {
                    entity: ev.entity,
                    action: packet.action.clone(),
                    position: IVec3::new(packet.block_position.x, packet.block_position.y, packet.block_position.z),
                    face: packet.face,
                    slot: None,
                });
            }
            BedrockProtocol::InventoryTransactionPacket(packet) => {
                if let InventoryTransactionData::ItemUse(data) = &packet.data
                    && data.action_type as u32 == ItemUseInventoryTransactionType::Place as u32
                {
                    action_writer.write(BlockActionMessage {
                        entity: ev.entity,
                        action: PlayerActionType::StartItemUseOn,
                        position: IVec3::new(data.block_position.x, data.block_position.y, data.block_position.z),
                        face: data.block_face as i32,
                        slot: u8::try_from(data.hotbar_slot).ok(),
                    });
                }
            }
            // only populated when the client runs with server authoritative block breaking
            BedrockProtocol::PlayerAuthInputPacket(packet) => {
                if let Some(transaction) = &packet.item_use_transaction
                    && transaction.item_use.action_type as u32 == ItemUseInventoryTransactionType::Place as u32
                {
                    let data = &transaction.item_use;
                    action_writer.write(BlockActionMessage {
                        entity: ev.entity,
                        action: PlayerActionType::StartItemUseOn,
                        position: IVec3::new(data.block_position.x, data.block_position.y, data.block_position.z),
                        face: data.block_face as i32,
                        slot: u8::try_from(data.hotbar_slot).ok(),
                    });
                }
                for action in packet
                    .player_block_actions
                    .iter()
                    .flatten()
                    .filter(|action| !matches!(action.action_type, PlayerActionType::StartItemUseOn))
                {
                    action_writer.write(BlockActionMessage {
                        entity: ev.entity,
                        action: action.action_type.clone(),
                        position: IVec3::new(action.position.x, action.position.y, action.position.z),
                        face: action.facing,
                        slot: None,
                    });
                }
            }
            _ => {}
        }
    }
}
