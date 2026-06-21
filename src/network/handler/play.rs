use crate::entity::entity::Entity as PlayerEntity;
use crate::level::BlockUpdatedMessage;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use bedrock::protocol::v662::packets::{UpdateBlockPacket};
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::Query;
use tracing::{debug, warn};
use vek::{Vec2, Vec3};

pub fn on_enter_play(mut sessions: Query<&mut Session>, mut state_reader: MessageReader<SessionStateChangedMessage>) {
    for ev in state_reader.read() {
        if ev.to != SessionState::Play {
            continue;
        }
        let Ok(_session) = sessions.get_mut(ev.entity) else {
            continue;
        };

        debug!("on_enter_play");
    }
}

pub fn handle_play(mut packet_reader: MessageReader<PacketReceivedMessage>, mut query: Query<(&mut PlayerEntity, &Session)>) {
    for ev in packet_reader.read() {
        let Ok((mut entity, session)) = query.get_mut(ev.entity) else {
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
