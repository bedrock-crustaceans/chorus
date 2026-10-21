use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::level::DimensionId;
use crate::level::generator::dimension::Dimension;
use crate::level::level::Level;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::transport::Transports;
use crate::player::block_break::BlockBreaking;
use crate::player::chunk_view::ChunkView;
use crate::world::blob_cache::{ClientBlobCache, blob_id};
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
use bevy_ecs::prelude::{Local, Query, Resource};
use bevy_ecs::system::{In, Res};
use bevy_tasks::ComputeTaskPool;
use chorus_core::config::Config;
use chorus_core::schedule::{Budget, Scheduler, Wake};
use chorus_level::chunk::Chunk;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::debug;

struct ChunkPayload {
    source: Arc<Chunk>,
    sub_chunk_count: u32,
    sub_chunk_limit: u16,
    // every sub chunk, then all biomes, each with its id; this is what a caching client gets by id
    blobs: Vec<(u64, Arc<[u8]>)>,
    // border blocks and block entities, never cached
    tail: Vec<u8>,
    // the whole thing glued back together, only built once some client without the cache needs it
    full: OnceLock<Vec<u8>>,
}

impl ChunkPayload {
    fn len(&self) -> usize {
        self.blobs.iter().map(|(_, blob)| blob.len()).sum::<usize>() + self.tail.len()
    }

    fn full(&self) -> &[u8] {
        self.full.get_or_init(|| {
            let mut data = Vec::with_capacity(self.len());
            for (_, blob) in &self.blobs {
                data.extend_from_slice(blob);
            }
            data.extend_from_slice(&self.tail);
            data
        })
    }
}

type ChunkKey = (i32, i32, i32);

#[derive(Default, Clone, Copy)]
pub struct ViewStats {
    pub radius: f32,
    pub generated_radius: f32,
    pub sent_radius: f32,
    pub wanted: usize,
    pub generated: usize,
    pub in_flight: usize,
    pub ready: usize,
    pub queued: usize,
    pub sent: usize,
}

#[derive(Resource)]
pub struct ChunkSerializer {
    sender: Sender<(ChunkKey, ChunkPayload)>,
    receiver: Mutex<Receiver<(ChunkKey, ChunkPayload)>>,
    in_flight: HashSet<ChunkKey>,
    ready: HashMap<ChunkKey, Arc<ChunkPayload>>,
}

impl Default for ChunkSerializer {
    fn default() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver: Mutex::new(receiver),
            in_flight: HashSet::new(),
            ready: HashMap::new(),
        }
    }
}

impl ChunkSerializer {
    fn collect(&mut self) {
        let receiver = self.receiver.get_mut().expect("the receiver is never locked elsewhere");
        while let Ok((key, payload)) = receiver.try_recv() {
            self.in_flight.remove(&key);
            self.ready.insert(key, Arc::new(payload));
        }
    }

    fn request(&mut self, key: ChunkKey, chunk: Arc<Chunk>) {
        if self.ready.contains_key(&key) || !self.in_flight.insert(key) {
            return;
        }
        let pool = ComputeTaskPool::get();
        if pool.thread_num() == 0 {
            self.in_flight.remove(&key);
            self.ready.insert(key, Arc::new(build_payload(chunk)));
            return;
        }
        let sender = self.sender.clone();
        pool.spawn(async move {
            let _ = sender.send((key, build_payload(chunk)));
        })
        .detach();
    }

    pub fn view_stats(&self, view: &ChunkView, dimension: Option<&Dimension>) -> ViewStats {
        let Some(center) = view.center else { return ViewStats::default() };
        let mut stats = ViewStats::default();
        let (mut nearest_ungenerated, mut nearest_unsent) = (i32::MAX, i32::MAX);
        for dx in -view.radius..=view.radius {
            for dz in -view.radius..=view.radius {
                let distance = dx * dx + dz * dz;
                if distance > view.radius * view.radius {
                    continue;
                }
                let position = (center.0 + dx, center.1 + dz);
                stats.wanted += 1;
                if dimension.is_some_and(|dimension| dimension.get_chunk(position.0, position.1).is_some()) {
                    stats.generated += 1;
                } else {
                    nearest_ungenerated = nearest_ungenerated.min(distance);
                }
                if view.sent.contains(&position) {
                    stats.sent += 1;
                } else {
                    nearest_unsent = nearest_unsent.min(distance);
                }
            }
        }
        let covered = |nearest_missing: i32| if nearest_missing == i32::MAX { view.radius as f32 } else { (nearest_missing as f32).sqrt() };
        stats.radius = view.radius as f32;
        stats.generated_radius = covered(nearest_ungenerated);
        stats.sent_radius = covered(nearest_unsent);
        for &(x, z) in &view.pending {
            let key = (view.dimension, x, z);
            stats.in_flight += self.in_flight.contains(&key) as usize;
            stats.ready += self.ready.contains_key(&key) as usize;
        }
        stats.queued = view.pending.len();
        stats
    }

    fn current(&self, key: &ChunkKey, dimension: &Dimension) -> bool {
        self.ready.get(key).is_some_and(|payload| dimension.holds_chunk(key.1, key.2, &payload.source))
    }

    fn retain_wanted(&mut self, wanted: &HashSet<ChunkKey>) {
        self.ready.retain(|key, _| wanted.contains(key));
    }
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
const MAX_OUTSTANDING_REQUESTS: usize = 1024;
const SCAN_WINDOW: usize = MAX_OUTSTANDING_REQUESTS * 2;
const UNLOAD_INTERVAL_TICKS: u32 = 20;

pub fn unload_distant_chunks(views: Query<&ChunkView>, mut level: ResMut<Level>, mut ticks: Local<u32>) {
    *ticks += 1;
    if *ticks < UNLOAD_INTERVAL_TICKS {
        return;
    }
    *ticks = 0;

    for dimension in level.dimensions_mut() {
        let id = dimension.id();
        let mut kept: HashSet<(i32, i32)> = HashSet::new();
        for view in views.iter().filter(|view| view.dimension == id) {
            let Some((center_x, center_z)) = view.center else { continue };
            let radius = view.radius + UNLOAD_MARGIN;
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    if dx * dx + dz * dz <= radius * radius {
                        kept.insert((center_x + dx, center_z + dz));
                    }
                }
            }
        }
        let unloaded = dimension.unload_chunks(|x, z| kept.contains(&(x, z)));
        if unloaded > 0 {
            debug!("unloaded {unloaded} chunks no player is near in {}", dimension.name());
        }
    }
}

pub fn request_view_chunks(
    mut query: Query<(&mut Session, &Transform, &mut ChunkView, Option<&mut ClientBlobCache>)>,
    mut level: ResMut<Level>,
    mut serializer: ResMut<ChunkSerializer>,
    mut scheduler: ResMut<Scheduler>,
) {
    let mut focus: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();
    let mut to_request: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();

    for (_, _, mut view, _) in query.iter_mut() {
        let Some(center) = view.center else { continue };

        focus.entry(view.dimension).or_default().push(center);

        let room = MAX_OUTSTANDING_REQUESTS.saturating_sub(view.requested.len()).min(MAX_NEW_REQUESTS_PER_TICK);
        let mut new_requests = Vec::new();
        for &(x, z) in view.pending.iter().take(SCAN_WINDOW) {
            if new_requests.len() >= room {
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

    if !serializer.ready.is_empty() {
        let wanted: HashSet<ChunkKey> = query
            .iter()
            .flat_map(|(_, _, view, _)| {
                let dimension_id = view.dimension;
                view.requested.iter().map(move |&(x, z)| (dimension_id, x, z))
            })
            .collect();
        serializer.retain_wanted(&wanted);
    }

    scheduler.wake::<ChunkSendTask>();
}

pub struct ChunkSendTask;

pub fn transmit_pending_chunks(
    _budget: In<Budget>,
    config: Res<Config>,
    transports: Transports,
    mut query: Query<(&mut Session, &Transform, &mut ChunkView, Option<&mut ClientBlobCache>)>,
    level: Res<Level>,
    mut serializer: ResMut<ChunkSerializer>,
    mut average_chunk_bytes: Local<usize>,
) -> Wake {
    serializer.collect();
    let mut selections: Vec<Vec<(i32, i32)>> = Vec::new();
    for (session, _, view, _) in query.iter() {
        let Some(dimension) = level.dimension(view.dimension) else {
            selections.push(Vec::new());
            continue;
        };
        let dimension_id = view.dimension;
        let limit = chunk_allowance(transports.send_capacity(&session.id), *average_chunk_bytes);
        let selection = take_ready(&view.pending, limit, |(x, z)| serializer.current(&(dimension_id, x, z), dimension));

        let mut lookahead = limit;
        for &(x, z) in view.pending.iter().take(SCAN_WINDOW) {
            if lookahead == 0 {
                break;
            }
            let key = (dimension_id, x, z);
            if serializer.current(&key, dimension) || serializer.in_flight.contains(&key) {
                lookahead -= 1;
                continue;
            }
            serializer.ready.remove(&key);
            if let Some(chunk) = dimension.chunk_handle(x, z) {
                serializer.request(key, chunk);
                lookahead -= 1;
            }
        }
        selections.push(selection);
    }

    let mut sent_keys: HashSet<ChunkKey> = HashSet::new();
    for ((mut session, transform, mut view, mut cache), selection) in query.iter_mut().zip(selections) {
        let dimension_id = view.dimension;
        let mut sent: Vec<(i32, i32)> = Vec::new();

        for (x, z) in selection {
            let Some(payload) = serializer.ready.get(&(dimension_id, x, z)).cloned() else { continue };

            let cached = if config.network.chunks.cache {
                cache.as_mut().filter(|cache| cache.has_room(payload.blobs.len()))
            } else {
                None
            };

            let (cache_blobs, serialized_chunk_data) = match cached {
                Some(cache) => {
                    cache.track(&payload.blobs);
                    (payload.blobs.iter().map(|(id, _)| *id).collect(), payload.tail.clone())
                }
                None => (vec![], if config.network.chunks.subchunks { payload.tail.clone() } else { payload.full().to_vec() }),
            };

            session.send(BedrockProtocol::LevelChunkPacket(
                LevelChunkPacket {
                    chunk_position: ChunkPos { x, z },
                    dimension_id: view.dimension,
                    sub_chunk_count: if config.network.chunks.subchunks { 0 } else { payload.sub_chunk_count },
                    client_request_sub_chunk_limit: config.network.chunks.subchunks.then_some(payload.sub_chunk_limit as i32),
                    cache_enabled: !cache_blobs.is_empty(),
                    cache_blobs,
                    serialized_chunk_data,
                }
                .into(),
            ));

            sent.push((x, z));
            sent_keys.insert((dimension_id, x, z));
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

    if !sent_keys.is_empty() {
        let total_bytes: usize = sent_keys.iter().filter_map(|key| serializer.ready.get(key)).map(|payload| payload.len()).sum();
        *average_chunk_bytes = total_bytes / sent_keys.len();
    }

    Wake::Done
}

fn chunk_allowance(capacity: Option<usize>, average_chunk_bytes: usize) -> usize {
    match capacity {
        None => usize::MAX,
        Some(0) => 0,
        Some(capacity) => (capacity / average_chunk_bytes.max(1)).max(1),
    }
}

fn take_ready(pending: &VecDeque<(i32, i32)>, limit: usize, is_ready: impl Fn((i32, i32)) -> bool) -> Vec<(i32, i32)> {
    let mut taken = Vec::new();
    for &position in pending.iter().take(SCAN_WINDOW) {
        if taken.len() >= limit {
            break;
        }
        if is_ready(position) {
            taken.push(position);
        }
    }
    taken
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
            server_built_chunks: Vec::new(),
        }
        .into(),
    ));
}

fn build_payload(chunk: Arc<Chunk>) -> ChunkPayload {
    let mut blobs: Vec<Vec<u8>> = chunk
        .sub_chunks()
        .iter()
        .enumerate()
        .map(|(i, sub_chunk)| sub_chunk.serialize_network(chunk.min_sub_chunk_y().wrapping_add(i as i8)))
        .collect();
    blobs.push(chunk.sub_chunks().iter().flat_map(|sub_chunk| sub_chunk.serialize_biomes()).collect());

    let mut tail = vec![0u8]; // border blocks
    tail.extend(chunk.serialize_block_entities(None));

    ChunkPayload {
        sub_chunk_count: chunk.sub_chunk_count() as u32,
        sub_chunk_limit: chunk.highest_non_air_sub_chunk_y() as u16,
        blobs: blobs.into_iter().map(|blob| (blob_id(&blob), Arc::from(blob))).collect(),
        tail,
        full: OnceLock::new(),
        source: chunk,
    }
}

const MAX_SUB_CHUNKS_PER_REQUEST: usize = 256;

fn resolve_sub_chunk_position(center: (i32, i32, i32), offset: (i32, i32, i32)) -> Option<(i32, i8, i32)> {
    let x = center.0.checked_add(offset.0)?;
    let y = i8::try_from(center.1.checked_add(offset.1)?).ok()?;
    let z = center.2.checked_add(offset.2)?;
    Some((x, y, z))
}

pub fn handle_sub_chunk_request(mut reader: MessageReader<PacketReceivedMessage>, mut query: Query<(&mut Session, Option<&mut ClientBlobCache>)>, level: Res<Level>, config: Res<Config>) {
    let blobs_enabled = config.network.chunks.cache;
    let subchunks_enabled = config.network.chunks.subchunks;

    for ev in reader.read() {
        let BedrockProtocol::SubChunkRequestPacket(packet) = &ev.packet else {
            continue;
        };

        if !subchunks_enabled {
            continue;
        }

        let Ok((mut session, mut cache)) = query.get_mut(ev.entity) else {
            continue;
        };

        let Some(dim) = level.dimension(packet.dimension_type) else {
            continue;
        };

        let mut entries = Vec::with_capacity(packet.sub_chunk_pos_offsets.len().min(MAX_SUB_CHUNKS_PER_REQUEST));

        for offset in packet.sub_chunk_pos_offsets.iter().take(MAX_SUB_CHUNKS_PER_REQUEST) {
            let (result, raw_data) = match resolve_sub_chunk_position(packet.center_pos, (offset.offset_x as i32, offset.offset_y as i32, offset.offset_z as i32)) {
                None => (SubChunkRequestResult::IndexOutOfBounds, None),
                Some((cx, cy, cz)) => match dim.get_chunk(cx, cz) {
                    None => (SubChunkRequestResult::LevelChunkDoesntExist, None),
                    Some(chunk) => match chunk.get_sub_chunk(cy) {
                        None => (SubChunkRequestResult::IndexOutOfBounds, None),
                        Some(sc) if sc.is_all_air() => (SubChunkRequestResult::SuccessAllAir, None),
                        Some(sc) => {
                            let mut data = sc.serialize_network(cy);
                            data.extend(chunk.serialize_block_entities(Some(cy)));
                            (SubChunkRequestResult::Success, Some(data))
                        }
                    },
                },
            };

            let use_blobs = blobs_enabled && cache.is_some() && raw_data.is_some();
            let (blob_id, serialized_sub_chunk) = if use_blobs {
                let data = raw_data.as_ref().unwrap();
                let id = blob_id(data);

                if let Some(cache) = cache.as_deref_mut() {
                    cache.track(&[(id, Arc::from(data.clone()))]);
                }
                (Some(id), None)
            } else {
                (None, raw_data)
            };

            entries.push(SubChunkDataEntry {
                sub_chunk_pos_offset: offset.clone(),
                sub_chunk_request_result: result,
                serialized_sub_chunk,
                height_map_data_type: HeightMapDataType::NoData,
                height_map_data: None,
                render_height_map_data_type: HeightMapDataType::NoData,
                render_height_map_data: None,
                blob_id,
            });
        }

        session.send(BedrockProtocol::SubChunkPacket(
            SubChunkPacket {
                cache_enabled: blobs_enabled,
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

#[cfg(test)]
mod tests {
    use super::*;

    use bevy_tasks::TaskPool;
    use std::time::{Duration, Instant};

    fn small_chunk() -> Arc<Chunk> {
        Arc::new(Chunk::new(0, 0, -4, 24, 0, 1))
    }

    fn wait_for_ready(serializer: &mut ChunkSerializer, key: ChunkKey) {
        let start = Instant::now();
        while !serializer.ready.contains_key(&key) {
            serializer.collect();
            assert!(start.elapsed() < Duration::from_secs(20), "the payload never arrived");
            std::thread::yield_now();
        }
    }

    #[test]
    fn take_ready_only_looks_at_the_scan_window() {
        let pending: VecDeque<(i32, i32)> = (0..SCAN_WINDOW as i32 + 10).map(|x| (x, 0)).collect();
        let ready = take_ready(&pending, usize::MAX, |_| true);
        assert_eq!(ready.len(), SCAN_WINDOW);
    }

    #[test]
    fn a_requested_chunk_is_serialized_on_the_pool_and_collected() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut serializer = ChunkSerializer::default();
        let chunk = small_chunk();
        serializer.request((0, 0, 0), chunk.clone());
        assert!(serializer.in_flight.contains(&(0, 0, 0)) || serializer.ready.contains_key(&(0, 0, 0)));
        wait_for_ready(&mut serializer, (0, 0, 0));
        assert!(serializer.in_flight.is_empty());
        assert!(Arc::ptr_eq(&serializer.ready[&(0, 0, 0)].source, &chunk));
    }

    #[test]
    fn requesting_a_chunk_twice_builds_one_payload() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut serializer = ChunkSerializer::default();
        let chunk = small_chunk();
        serializer.request((0, 1, 1), chunk.clone());
        serializer.request((0, 1, 1), chunk.clone());
        assert!(serializer.in_flight.len() <= 1);
        wait_for_ready(&mut serializer, (0, 1, 1));
        let first = serializer.ready[&(0, 1, 1)].clone();
        serializer.request((0, 1, 1), chunk);
        assert!(Arc::ptr_eq(&first, &serializer.ready[&(0, 1, 1)]));
        assert!(serializer.in_flight.is_empty());
    }

    #[test]
    fn payloads_nobody_wants_any_more_are_dropped() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut serializer = ChunkSerializer::default();
        for key in [(0, 2, 2), (0, 3, 3)] {
            serializer.request(key, small_chunk());
            wait_for_ready(&mut serializer, key);
        }
        serializer.retain_wanted(&HashSet::from([(0, 3, 3)]));
        assert!(!serializer.ready.contains_key(&(0, 2, 2)));
        assert!(serializer.ready.contains_key(&(0, 3, 3)));
    }

    #[test]
    fn a_payload_built_from_the_same_chunk_matches_a_direct_build() {
        let chunk = small_chunk();
        let first = build_payload(chunk.clone());
        let second = build_payload(chunk);
        assert_eq!(first.len(), second.len());
        assert_eq!(first.full(), second.full());
        assert_eq!(first.sub_chunk_count, second.sub_chunk_count);
    }

    #[test]
    fn resolves_in_range_positions() {
        assert_eq!(resolve_sub_chunk_position((10, 0, -3), (1, -4, 2)), Some((11, -4, -1)));
    }

    #[test]
    fn rejects_overflow_and_out_of_range_y() {
        assert_eq!(resolve_sub_chunk_position((i32::MAX, 0, 0), (1, 0, 0)), None);
        assert_eq!(resolve_sub_chunk_position((0, 0, i32::MIN), (0, 0, -1)), None);
        assert_eq!(resolve_sub_chunk_position((0, 256, 0), (0, 0, 0)), None);
        assert_eq!(resolve_sub_chunk_position((0, 100, 0), (0, 100, 0)), None);
    }

    #[test]
    fn the_allowance_follows_the_transport_capacity() {
        assert_eq!(chunk_allowance(None, 1000), usize::MAX);
        assert_eq!(chunk_allowance(Some(0), 1000), 0);
        assert_eq!(chunk_allowance(Some(10_500), 1000), 10);
        assert_eq!(chunk_allowance(Some(10), 1000), 1);
        assert_eq!(chunk_allowance(Some(5000), 0), 5000);
    }

    #[test]
    fn the_sent_radius_stops_at_the_nearest_chunk_that_has_not_been_sent() {
        let mut view = ChunkView {
            radius: 6,
            center: Some((0, 0)),
            ..Default::default()
        };
        for x in -6..=6 {
            for z in -6..=6 {
                if x * x + z * z <= 36 && (x, z) != (3, 4) {
                    view.sent.insert((x, z));
                }
            }
        }
        let stats = ChunkSerializer::default().view_stats(&view, None);
        assert_eq!(stats.radius, 6.0);
        assert_eq!(stats.sent_radius, 5.0);
        assert_eq!(stats.generated_radius, 0.0);
        view.sent.insert((3, 4));
        assert_eq!(ChunkSerializer::default().view_stats(&view, None).sent_radius, 6.0);
    }

    #[test]
    fn take_ready_keeps_queue_order_and_stops_at_limit() {
        let pending: VecDeque<(i32, i32)> = (0..10).map(|x| (x, 0)).collect();
        let taken = take_ready(&pending, 3, |(x, _)| x % 2 == 1);
        assert_eq!(taken, vec![(1, 0), (3, 0), (5, 0)]);
    }

    #[test]
    fn take_ready_skips_unready_and_handles_short_queues() {
        let pending: VecDeque<(i32, i32)> = (0..4).map(|x| (x, 1)).collect();
        assert!(take_ready(&pending, 8, |_| false).is_empty());
        assert_eq!(take_ready(&pending, 8, |_| true).len(), 4);
        assert!(take_ready(&pending, 0, |_| true).is_empty());
    }
}
