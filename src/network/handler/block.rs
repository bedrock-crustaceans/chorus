use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::world::block::BlockActionMessage;
use bevy_ecs::message::{MessageReader, MessageWriter};
use glam::IVec3;

pub fn decode_block_actions(mut packet_reader: MessageReader<PacketReceivedMessage>, mut action_writer: MessageWriter<BlockActionMessage>) {
    for ev in packet_reader.read() {
        match &ev.packet {
            BedrockProtocol::PlayerActionPacket(packet) => {
                action_writer.write(BlockActionMessage {
                    entity: ev.entity,
                    action: packet.action.clone(),
                    position: IVec3::new(packet.block_position.x, packet.block_position.y, packet.block_position.z),
                    face: packet.face,
                });
            }
            // only populated when the client runs with server authoritative block breaking
            BedrockProtocol::PlayerAuthInputPacket(packet) => {
                for action in packet.player_block_actions.iter().flatten() {
                    action_writer.write(BlockActionMessage {
                        entity: ev.entity,
                        action: action.action_type.clone(),
                        position: IVec3::new(action.position.x, action.position.y, action.position.z),
                        face: action.facing,
                    });
                }
            }
            _ => {}
        }
    }
}
