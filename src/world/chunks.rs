use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::level::DimensionId;
use crate::level::generator::dimension::Dimension;
use crate::level::level::Level;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::player::block_break::BlockBreaking;
use crate::player::chunk_view::ChunkView;
use bedrock::protocol::v662::enums::{PlayStatus, PlayerActionType};
use bedrock::protocol::v662::packets::{NetworkChunkPublisherUpdatePacket, PlayerActionPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, BlockPos, ChunkPos};
use bedrock::protocol::v712::packets::ChangeDimensionPacket;
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bedrock::protocol::v2168::packets::LevelChunkPacket;
use bedrock::protocol::v2168::types::SubChunkPos;
use bedrock::protocol::v2193::packets::{HeightMapDataType, SubChunkDataEntry, SubChunkPacket, SubChunkRequestResult};
use bevy_ecs::change_detection::ResMut;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{Local, Query};
use bevy_ecs::system::Res;
use bevy_tasks::ComputeTaskPool;
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::debug;

struct ChunkPayload {
    sub_chunk_count: u32,
    sub_chunk_limit: u16,
    data: Vec<u8>,
}

pub fn update_chunk_order(mut query: Query<(&mut Session, &Transform, &mut ChunkView, &mut BlockBreaking, &ActorId, &DimensionId)>, mut level: ResMut<Level>) {
    let mut left_view: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();
    for (mut session, transform, mut view, mut breaking, actor, dimension) in query.iter_mut() {
        if view.dimension != dimension.0 {
            breaking.current = None;
            switch_dimension(&mut session, transform, &mut view, actor, dimension.0, &mut left_view);
        }

        if view.radius == 0 {
            continue;
        }

        let center = (transform.position.x.floor() as i32 >> 4, transform.position.z.floor() as i32 >> 4);
        if view.center == Some(center) {
            continue;
        }

        view.center = Some(center);

        let radius = view.radius;
        let mut wanted: Vec<(i32, i32)> = Vec::new();

        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx * dx + dz * dz > radius * radius {
                    continue;
                }

                wanted.push((center.0 + dx, center.1 + dz));
            }
        }

        // nearest first, so the ground under the player fills in before the edges of the view
        wanted.sort_unstable_by_key(|&(x, z)| (x - center.0).pow(2) + (z - center.1).pow(2));

        // the client drops whatever falls outside the published radius, so it has to be re-sent
        // if the player ever comes back
        let in_view: HashSet<(i32, i32)> = wanted.iter().copied().collect();
        view.sent.retain(|position| in_view.contains(position));
        let left = view.requested.iter().copied().filter(|position| !in_view.contains(position));
        left_view.entry(view.dimension).or_default().extend(left);
        view.requested.retain(|position| in_view.contains(position));

        let pending: VecDeque<(i32, i32)> = wanted.into_iter().filter(|position| !view.sent.contains(position)).collect();
        view.pending = pending;

        send_publisher_update(&mut session, transform, &view);
    }

    for (id, mut positions) in left_view {
        positions.retain(|position| query.iter().all(|(_, _, view, _, _, _)| view.dimension != id || !view.requested.contains(position)));
        if let Some(dimension) = level.dimension_mut(id)
            && !positions.is_empty()
        {
            dimension.cancel_chunks(&positions);
        }
    }
}

fn switch_dimension(session: &mut Session, transform: &Transform, view: &mut ChunkView, actor: &ActorId, target: i32, abandoned: &mut HashMap<i32, Vec<(i32, i32)>>) {
    let requested = std::mem::take(&mut view.requested);
    abandoned.entry(view.dimension).or_default().extend(requested);
    view.sent.clear();
    view.pending.clear();
    view.center = None;
    view.dimension = target;
    view.dimension_changes += 1;

    session.send(BedrockProtocol::ChangeDimensionPacket(
        ChangeDimensionPacket {
            dimension_id: target,
            position: (transform.position.x, transform.position.y, transform.position.z),
            respawn: false,
            loading_screen_id: Some(view.dimension_changes),
        }
        .into(),
    ));
    session.send_play_status(PlayStatus::PlayerSpawn, false);

    let origin = || NetworkBlockPosition { x: 0, y: 0, z: 0 };
    session.send(BedrockProtocol::PlayerActionPacket(
        PlayerActionPacket {
            player_runtime_id: ActorRuntimeID(actor.runtime_id),
            action: PlayerActionType::ChangeDimensionAck,
            block_position: origin(),
            result_pos: origin(),
            face: 0,
        }
        .into(),
    ));
}

const UNLOAD_MARGIN: i32 = 2;
const MAX_NEW_REQUESTS_PER_TICK: usize = 256;
const UNLOAD_INTERVAL_TICKS: u32 = 20;

pub fn unload_distant_chunks(views: Query<&ChunkView>, mut level: ResMut<Level>, mut ticks: Local<u32>) {
    *ticks += 1;
    if *ticks < UNLOAD_INTERVAL_TICKS {
        return;
    }
    *ticks = 0;

    for dimension in level.dimensions_mut() {
        let id = dimension.id();
        let nearby: Vec<((i32, i32), i32)> = views
            .iter()
            .filter(|view| view.dimension == id)
            .filter_map(|view| Some((view.center?, view.radius + UNLOAD_MARGIN)))
            .collect();
        let unloaded = dimension.unload_chunks(|x, z| nearby.iter().any(|&((center_x, center_z), radius)| (x - center_x).pow(2) + (z - center_z).pow(2) <= radius * radius));
        if unloaded > 0 {
            debug!("unloaded {unloaded} chunks no player is near in {}", dimension.name());
        }
    }
}

pub fn send_pending_chunks(mut query: Query<(&mut Session, &Transform, &mut ChunkView)>, mut level: ResMut<Level>) {
    let mut focus: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();
    let mut to_request: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();
    for (_, _, mut view) in query.iter_mut() {
        let Some(center) = view.center else { continue };
        focus.entry(view.dimension).or_default().push(center);

        let mut new_requests = Vec::new();
        for &(x, z) in &view.pending {
            if new_requests.len() >= MAX_NEW_REQUESTS_PER_TICK {
                break;
            }
            if (x - center.0).pow(2) + (z - center.1).pow(2) <= view.radius.pow(2) && !view.requested.contains(&(x, z)) {
                new_requests.push((x, z));
            }
        }
        view.requested.extend(new_requests.iter().copied());
        to_request.entry(view.dimension).or_default().extend(new_requests);
    }

    for (id, centers) in focus {
        if let Some(dimension) = level.dimension_mut(id) {
            dimension.set_focus(&centers);
        }
    }
    for (id, positions) in to_request {
        if let Some(dimension) = level.dimension_mut(id) {
            dimension.request_chunks(&positions);
        }
    }

    let mut ready: HashMap<i32, HashSet<(i32, i32)>> = HashMap::new();
    for (_, _, view) in query.iter() {
        let Some(dimension) = level.dimension(view.dimension) else { continue };
        let ready = ready.entry(view.dimension).or_default();
        ready.extend(view.pending.iter().copied().filter(|&(x, z)| dimension.get_chunk(x, z).is_some()));
    }

    let payloads: HashMap<i32, HashMap<(i32, i32), ChunkPayload>> = ready
        .into_iter()
        .filter(|(_, positions)| !positions.is_empty())
        .filter_map(|(id, positions)| {
            let positions: Vec<(i32, i32)> = positions.into_iter().collect();
            Some((id, serialize_chunks(level.dimension(id)?, &positions)))
        })
        .collect();
    if payloads.is_empty() {
        return;
    }

    for (mut session, transform, mut view) in query.iter_mut() {
        let Some(payloads) = payloads.get(&view.dimension) else { continue };
        let mut sent: Vec<(i32, i32)> = Vec::new();

        for &(x, z) in &view.pending {
            let Some(payload) = payloads.get(&(x, z)) else { continue };

            session.send(BedrockProtocol::LevelChunkPacket(
                LevelChunkPacket {
                    chunk_position: ChunkPos { x, z },
                    dimension_id: view.dimension,
                    sub_chunk_count: payload.sub_chunk_count,
                    client_request_sub_chunk_limit: None,
                    cache_enabled: false,
                    cache_blobs: vec![],
                    serialized_chunk_data: payload.data.clone(),
                }
                .into(),
            ));

            sent.push((x, z));
        }

        if sent.is_empty() {
            continue;
        }

        for position in &sent {
            view.requested.remove(position);
            view.sent.insert(*position);
        }
        let sent: HashSet<(i32, i32)> = sent.into_iter().collect();
        view.pending.retain(|position| !sent.contains(position));

        send_publisher_update(&mut session, transform, &view);
    }
}

fn send_publisher_update(session: &mut Session, transform: &Transform, view: &ChunkView) {
    session.send(BedrockProtocol::NetworkChunkPublisherUpdatePacket(
        NetworkChunkPublisherUpdatePacket {
            new_view_position: BlockPos {
                x: transform.position.x.floor() as i32,
                y: transform.position.y.floor() as i32,
                z: transform.position.z.floor() as i32,
            },
            new_view_radius: (view.radius as u32) << 4,
            server_built_chunks: view.sent.iter().map(|&(x, z)| ChunkPos { x, z }).collect(),
        }
        .into(),
    ));
}

fn serialize_chunks(dimension: &Dimension, positions: &[(i32, i32)]) -> HashMap<(i32, i32), ChunkPayload> {
    let min_y = dimension.dimension_type.min_sub_chunk_y();

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
                    Some(sc) => {
                        let mut data = sc.serialize_network(cy as i8);
                        data.extend(chunk.serialize_block_entities(Some(cy as i8)));
                        (SubChunkRequestResult::Success, Some(data))
                    }
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
