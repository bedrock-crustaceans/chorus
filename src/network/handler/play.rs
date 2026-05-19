use crate::entity::entity::Entity as PlayerEntity;
use crate::level::BlockUpdatedMessage;
use crate::level::level::Level;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::registry::block_registry::BlockRegistry;
use bedrock::protocol::v662::packets::{LevelChunkPacket, NetworkChunkPublisherUpdatePacket, UpdateBlockPacket};
use bedrock::protocol::v662::types::{BlockPos, ChunkPos};
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::Query;
use bevy_ecs::system::{Res, ResMut};
use tracing::{debug, warn};
use vek::{Vec2, Vec3};

pub fn on_enter_play(mut sessions: Query<&mut Session>, mut state_reader: MessageReader<SessionStateChangedMessage>, mut level: ResMut<Level>, registry: Res<BlockRegistry>) {
    for ev in state_reader.read() {
        if ev.to != SessionState::Play {
            continue;
        }
        let Ok(mut session) = sessions.get_mut(ev.entity) else {
            continue;
        };

        let radius = 8i32;
        let min_y = level.overworld().min_sub_chunk_y;

        session.send(BedrockProtocol::NetworkChunkPublisherUpdatePacket(
            NetworkChunkPublisherUpdatePacket {
                new_view_position: BlockPos { x: 0, y: 0, z: 0 },
                new_view_radius: (radius as u32) << 4,
                server_built_chunks: vec![],
            }
            .into(),
        ));

        for cx in -radius..radius {
            for cz in -radius..radius {
                let chunk = level.overworld_mut().get_or_generate_chunk(&registry, cx, cz);
                let limit = (chunk.highest_non_air_sub_chunk_y() - min_y) as u16;
                let biome_data = chunk.serialize_biomes();

                session.send(BedrockProtocol::LevelChunkPacket(
                    LevelChunkPacket {
                        chunk_position: ChunkPos { x: cx, z: cz },
                        dimension_id: 0,
                        sub_chunk_count: u32::MAX - 1,
                        sub_chunk_limit: limit,
                        cache_enabled: false,
                        cache_blobs: vec![],
                        serialized_chunk_data: biome_data,
                    }
                    .into(),
                ));
            }
        }

        debug!("on_enter_play: sent {} chunks for radius {}", (radius * 2).pow(2), radius);
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
                    block_runtime_id: msg.block_id,
                    flags: 0xB,
                    layer: msg.layer as u32,
                }
                .into(),
            ));
        }
    }
}
