use crate::entity::entity::Entity as PlayerEntity;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::Query;
use tracing::warn;
use vek::{Vec2, Vec3};

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
