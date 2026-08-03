use crate::level::dimension::Dimension;
use crate::level::level::Level;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::player::Player;
use crate::registry::block_registry::BlockRegistry;
use bedrock::protocol::v662::packets::{LevelChunkPacket, NetworkChunkPublisherUpdatePacket};
use bedrock::protocol::v662::types::{BlockPos, ChunkPos, SubChunkPos};
use bedrock::protocol::v818::packets::{HeightMapDataType, SubChunkDataEntry, SubChunkPacket, SubChunkRequestResult};
use bevy_ecs::change_detection::ResMut;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{Entity, Query};
use bevy_ecs::system::Res;
use bevy_tasks::ComputeTaskPool;
use std::collections::HashMap;
use tracing::debug;

const MAX_CHUNKS_PER_TICK: usize = 16;

struct ChunkPayload {
    sub_chunk_count: u32,
    sub_chunk_limit: u16,
    data: Vec<u8>,
}

pub fn send_pending_chunks(mut query: Query<(Entity, &mut Session, &mut Player)>, mut level: ResMut<Level>, registry: Res<BlockRegistry>) {
    let mut batches: HashMap<Entity, Vec<(i32, i32)>> = HashMap::new();

    for (entity, _, mut player) in query.iter_mut() {
        if player.chunks_radius == 0 {
            continue;
        }

        let count = player.chunks_pending.len().min(MAX_CHUNKS_PER_TICK);
        if count == 0 {
            continue;
        }

        let batch: Vec<(i32, i32)> = player.chunks_pending.drain(..count).collect();
        player.chunks_sent.extend(batch.iter().copied());

        batches.insert(entity, batch);
    }

    if batches.is_empty() {
        return;
    }

    // the same chunk is often requested by several players, so generate and serialize it once
    let mut positions: Vec<(i32, i32)> = batches.values().flatten().copied().collect();
    positions.sort_unstable();
    positions.dedup();

    let overworld = level.overworld_mut();
    overworld.generate_chunks(&registry, &positions);

    let payloads = serialize_chunks(overworld, &positions);

    for (entity, mut session, player) in query.iter_mut() {
        let Some(batch) = batches.get(&entity) else { continue };

        for &(x, z) in batch {
            let Some(payload) = payloads.get(&(x, z)) else { continue };

            session.send(BedrockProtocol::LevelChunkPacket(
                LevelChunkPacket {
                    chunk_position: ChunkPos { x, z },
                    dimension_id: 0,
                    sub_chunk_count: payload.sub_chunk_count,
                    sub_chunk_limit: payload.sub_chunk_limit,
                    cache_enabled: false,
                    cache_blobs: vec![],
                    serialized_chunk_data: payload.data.clone(),
                }
                .into(),
            ));

            debug!("sent chunk {}, {}", x, z);
        }

        if player.chunks_pending.is_empty() {
            session.send(BedrockProtocol::NetworkChunkPublisherUpdatePacket(
                NetworkChunkPublisherUpdatePacket {
                    new_view_position: BlockPos { x: 0, y: 0, z: 0 },
                    new_view_radius: (player.chunks_radius as u32) << 4,
                    server_built_chunks: player.chunks_sent.iter().map(|&(x, z)| ChunkPos { x, z }).collect(),
                }
                .into(),
            ));
        }
    }
}

fn serialize_chunks(dimension: &Dimension, positions: &[(i32, i32)]) -> HashMap<(i32, i32), ChunkPayload> {
    let min_y = dimension.min_sub_chunk_y;

    let serialized = ComputeTaskPool::get().scope(|scope| {
        for &(x, z) in positions {
            let Some(chunk) = dimension.get_chunk(x, z) else { continue };

            scope.spawn(async move {
                (
                    (x, z),
                    ChunkPayload {
                        sub_chunk_count: chunk.sub_chunk_count() as u32,
                        sub_chunk_limit: (chunk.highest_non_air_sub_chunk_y() - min_y) as u16,
                        data: chunk.serialize(),
                    },
                )
            });
        }
    });

    serialized.into_iter().collect()
}

pub fn handle_sub_chunk_request(mut reader: MessageReader<PacketReceivedMessage>, mut query: Query<&mut Session>, level: Res<Level>) {
    for ev in reader.read() {
        let BedrockProtocol::SubChunkRequestPacket(packet) = &ev.packet else {
            continue;
        };
        let Ok(mut session) = query.get_mut(ev.entity) else {
            continue;
        };

        debug!(
            "SubChunkRequestPacket: dim={} center=({},{},{}) offsets={}",
            packet.dimension_type,
            packet.center_pos.0,
            packet.center_pos.1,
            packet.center_pos.2,
            packet.sub_chunk_pos_offsets.len()
        );

        let mut entries = Vec::with_capacity(packet.sub_chunk_pos_offsets.len());

        for offset in &packet.sub_chunk_pos_offsets {
            let cx = packet.center_pos.0 + offset.offset_x as i32;
            let cy = packet.center_pos.1 + offset.offset_y as i32;
            let cz = packet.center_pos.2 + offset.offset_z as i32;

            let dim = level.dimension(packet.dimension_type);
            let chunk = dim.and_then(|d| d.get_chunk(cx, cz));

            let (result, data) = match (chunk, dim) {
                (Some(chunk), Some(_)) => match chunk.get_sub_chunk(cy as i8) {
                    None => (SubChunkRequestResult::SuccessAllAir, None),
                    Some(sc) if sc.is_all_air() => (SubChunkRequestResult::SuccessAllAir, None),
                    Some(sc) => (SubChunkRequestResult::Success, Some(sc.serialize_network(cy as i8))),
                },
                _ => (SubChunkRequestResult::LevelChunkDoesntExist, None),
            };

            entries.push(SubChunkDataEntry {
                sub_chunk_pos_offset: offset.clone(),
                sub_chunk_request_result: result,
                serialized_sub_chunk: Some(data.unwrap_or_default()),
                height_map_data_type: HeightMapDataType::NoData,
                height_map_data: None,
                render_height_map_data_type: HeightMapDataType::NoData,
                render_height_map_data: None,
                blob_id: None,
            });
        }

        session.send(BedrockProtocol::SubChunkPacket(
            SubChunkPacket {
                cache_enabled: false,
                dimension_type: packet.dimension_type,
                center_pos: SubChunkPos {
                    x: packet.center_pos.0,
                    y: packet.center_pos.1,
                    z: packet.center_pos.2,
                },
                sub_chunk_data: entries,
            }
            .into(),
        ));
    }
}
