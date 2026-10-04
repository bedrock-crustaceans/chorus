use crate::chunk::{BlockEntity, Chunk};
use crate::dimension_type::DimensionType;
use crate::sub_chunk::SubChunk;
use bedrock::level::db::{CompactionMode, Database, OpenOptions, WriteBatch};
use chorus_block::block_registry::BlockRegistry;
use chorus_block::hash_utils::HashUtils;
use glam::IVec3;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::error;

const CHUNK_VERSION: u8 = 40;
const SUB_CHUNK_VERSION: u8 = 9;
const STORAGE_VERSION: i32 = 10;
const FINALIZED: i32 = 2;
const PLAINS: i32 = 1;
const POOL_COMPACTION_SLICE: Duration = Duration::from_millis(50);
const TICK_COMPACTION_BUDGET: Duration = Duration::from_millis(10);

const TAG_DATA_3D: u8 = 0x2b;
const TAG_VERSION: u8 = 0x2c;
const TAG_SUB_CHUNK: u8 = 0x2f;
const TAG_BLOCK_ENTITY: u8 = 0x31;
const TAG_FINALIZED_STATE: u8 = 0x36;

#[derive(Debug)]
pub enum StorageError {
    Database(String),
    Io(std::io::Error),
    Nbt(nbtx::Error),
    Malformed(&'static str),
}

impl Display for StorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(message) => write!(f, "database error: {message}"),
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Nbt(error) => write!(f, "nbt error: {error}"),
            Self::Malformed(what) => write!(f, "malformed {what}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<nbtx::Error> for StorageError {
    fn from(error: nbtx::Error) -> Self {
        Self::Nbt(error)
    }
}

fn database_error(error: impl Display) -> StorageError {
    StorageError::Database(error.to_string())
}

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Clone, Debug)]
pub struct LevelData {
    pub name: String,
    pub seed: i64,
    pub spawn: IVec3,
}

type PendingWrite = (Vec<u8>, Option<Arc<[u8]>>);
type PendingWrites = HashMap<Vec<u8>, Option<Arc<[u8]>>>;

fn io_pool() -> Option<&'static bevy_tasks::TaskPool> {
    bevy_tasks::IoTaskPool::try_get()
        .map(|pool| &**pool)
        .filter(|pool| pool.thread_num() > 0)
        .or_else(|| bevy_tasks::AsyncComputeTaskPool::try_get().map(|pool| &**pool).filter(|pool| pool.thread_num() > 0))
}

pub fn spawn_io(work: impl FnOnce() + Send + 'static) {
    match io_pool() {
        Some(pool) => pool.spawn(async move { work() }).detach(),
        None => work(),
    }
}

pub struct LevelStorage {
    path: PathBuf,
    db: Mutex<Database>,
    pending: Mutex<PendingWrites>,
    flushing: AtomicBool,
    compacting: AtomicBool,
    air_id: i32,
    block_nbt: HashMap<i32, Vec<u8>>,
    block_ids: Mutex<HashMap<Vec<u8>, i32>>,
}

impl LevelStorage {
    pub fn open(path: impl AsRef<Path>, registry: &BlockRegistry, compression_level: u8) -> StorageResult<Self> {
        let path = path.as_ref().to_path_buf();
        let db_path = path.join("db");
        std::fs::create_dir_all(&db_path)?;
        let options = OpenOptions {
            compaction_mode: CompactionMode::Manual,
            compression_level,
        };
        let db = Database::open_with(db_path.to_string_lossy(), options).map_err(database_error)?;

        let block_nbt = registry
            .permutations()
            .map(|permutation| Ok((permutation.get_hash(), nbtx::to_le_bytes(&permutation.to_nbt())?)))
            .collect::<StorageResult<HashMap<_, _>>>()?;

        Ok(Self {
            path,
            db: Mutex::new(db),
            pending: Mutex::new(HashMap::new()),
            flushing: AtomicBool::new(false),
            compacting: AtomicBool::new(false),
            air_id: registry.get_block_id("minecraft:air").expect("air is registered"),
            block_nbt,
            block_ids: Mutex::new(HashMap::new()),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn pending(&self) -> MutexGuard<'_, PendingWrites> {
        self.pending.lock().expect("pending writes lock poisoned")
    }

    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        if let Some(value) = self.pending().get(key) {
            return Ok(value.as_ref().map(|value| value.to_vec()));
        }
        let db = self.db.lock().expect("level database lock poisoned");
        Ok(db.get(key).map_err(database_error)?.map(Vec::from))
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) {
        self.pending().insert(key, Some(value.into()));
    }

    fn remove(&self, key: Vec<u8>) {
        self.pending().insert(key, None);
    }

    pub fn pending_writes(&self) -> usize {
        self.pending().len()
    }

    fn flush(&self) -> StorageResult<()> {
        let batch: Vec<PendingWrite> = self.pending().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
        if batch.is_empty() {
            return Ok(());
        }
        let mut write = WriteBatch::new();
        for (key, value) in &batch {
            match value {
                Some(value) => write.insert(key, value),
                None => write.remove(key),
            }
        }
        self.db.lock().expect("level database lock poisoned").write(write).map_err(database_error)?;
        let mut pending = self.pending();
        for (key, written) in batch {
            let unchanged = match (pending.get(&key), &written) {
                (Some(Some(current)), Some(written)) => Arc::ptr_eq(current, written),
                (Some(None), None) => true,
                _ => false,
            };
            if unchanged {
                pending.remove(&key);
            }
        }
        Ok(())
    }

    pub fn schedule_flush(self: &Arc<Self>) {
        if self.pending().is_empty() || self.flushing.swap(true, Ordering::AcqRel) {
            return;
        }
        let storage = self.clone();
        spawn_io(move || {
            loop {
                let result = storage.flush();
                storage.flushing.store(false, Ordering::Release);
                if let Err(err) = result {
                    error!("failed to write level data to disk: {err}");
                    return;
                }
                if storage.pending().is_empty() || storage.flushing.swap(true, Ordering::AcqRel) {
                    return;
                }
            }
        });
    }

    pub fn schedule_compaction(self: &Arc<Self>) {
        if self.compacting.swap(true, Ordering::AcqRel) || io_pool().is_none() {
            return;
        }
        let storage = self.clone();
        spawn_io(move || while storage.compact_step(POOL_COMPACTION_SLICE) {});
    }

    /// Runs one compaction round on this thread when a compaction was scheduled without any pool
    /// threads to hand it to. Called every tick so a single-threaded server never stalls on a full compaction.
    pub fn step_compaction(&self) {
        if io_pool().is_none() && self.compacting.load(Ordering::Acquire) {
            self.compact_step(TICK_COMPACTION_BUDGET);
        }
    }

    fn compact_step(&self, budget: Duration) -> bool {
        let more = match self.db.lock().expect("level database lock poisoned").compact_step(budget) {
            Ok(more) => more,
            Err(err) => {
                error!("failed to compact the level database: {err}");
                false
            }
        };
        if !more {
            self.compacting.store(false, Ordering::Release);
        }
        more
    }

    pub fn flush_blocking(&self) -> StorageResult<()> {
        while self.flushing.swap(true, Ordering::AcqRel) {
            std::thread::sleep(Duration::from_millis(5));
        }
        let result = self.flush();
        self.flushing.store(false, Ordering::Release);
        result
    }

    pub fn read_level_data(&self) -> StorageResult<Option<LevelData>> {
        let path = self.path.join("level.dat");
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(path)?;
        let mut body = bytes.get(8..).ok_or(StorageError::Malformed("level.dat header"))?;
        let tag: HashMap<String, nbtx::Value> = nbtx::from_le_bytes(&mut body)?;

        let int = |key: &str| match tag.get(key) {
            Some(nbtx::Value::Int(value)) => Some(*value),
            _ => None,
        };
        let seed = match tag.get("RandomSeed") {
            Some(nbtx::Value::Long(seed)) => *seed,
            _ => return Err(StorageError::Malformed("level.dat RandomSeed")),
        };
        let name = match tag.get("LevelName") {
            Some(nbtx::Value::String(name)) => name.to_string(),
            _ => String::new(),
        };
        let spawn = IVec3::new(int("SpawnX").unwrap_or(0), int("SpawnY").unwrap_or(i16::MAX as i32), int("SpawnZ").unwrap_or(0));
        Ok(Some(LevelData { name, seed, spawn }))
    }

    pub fn write_level_data(&self, data: &LevelData) -> StorageResult<()> {
        let last_played = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_secs() as i64);
        let mut tag = nbtx::Compound::new();
        tag.insert("LevelName".into(), nbtx::Value::String(data.name.as_str().into()));
        tag.insert("RandomSeed".into(), nbtx::Value::Long(data.seed));
        tag.insert("SpawnX".into(), nbtx::Value::Int(data.spawn.x));
        tag.insert("SpawnY".into(), nbtx::Value::Int(data.spawn.y));
        tag.insert("SpawnZ".into(), nbtx::Value::Int(data.spawn.z));
        tag.insert("StorageVersion".into(), nbtx::Value::Int(STORAGE_VERSION));
        tag.insert("Generator".into(), nbtx::Value::Int(1));
        tag.insert("LastPlayed".into(), nbtx::Value::Long(last_played));
        let body = nbtx::to_le_bytes(&nbtx::Value::Compound(tag))?;

        let mut bytes = Vec::with_capacity(body.len() + 8);
        bytes.extend(STORAGE_VERSION.to_le_bytes());
        bytes.extend((body.len() as i32).to_le_bytes());
        bytes.extend(body);
        std::fs::write(self.path.join("level.dat"), bytes)?;
        std::fs::write(self.path.join("levelname.txt"), &data.name)?;
        Ok(())
    }

    pub fn save_chunk(&self, dimension: DimensionType, chunk: &Chunk) -> StorageResult<()> {
        let id = dimension.id();
        let (x, z) = (chunk.x, chunk.z);
        self.put(key(id, x, z, TAG_VERSION, None), vec![CHUNK_VERSION]);

        for (offset, sub_chunk) in chunk.sub_chunks().iter().enumerate() {
            let index = chunk.min_sub_chunk_y().wrapping_add(offset as i8);
            let sub_key = key(id, x, z, TAG_SUB_CHUNK, Some(index));
            if sub_chunk.is_all_air() && sub_chunk.layers().iter().skip(1).all(|layer| layer.count(self.air_id) == 4096) {
                self.remove(sub_key);
            } else {
                self.put(sub_key, self.encode_sub_chunk(sub_chunk, index));
            }
        }

        let mut data_3d = Vec::with_capacity(512 + chunk.sub_chunk_count() * 8);
        for height in self.heightmap(chunk) {
            data_3d.extend(height.to_le_bytes());
        }
        for sub_chunk in chunk.sub_chunks() {
            sub_chunk.biomes().write_biomes_disk(&mut data_3d);
        }
        self.put(key(id, x, z, TAG_DATA_3D, None), data_3d);

        let entities_key = key(id, x, z, TAG_BLOCK_ENTITY, None);
        if chunk.block_entities().is_empty() {
            self.remove(entities_key);
        } else {
            let mut entities = Vec::new();
            for entity in chunk.block_entities() {
                nbtx::to_le_bytes_in(&mut entities, &entity.data)?;
            }
            self.put(entities_key, entities);
        }

        self.put(key(id, x, z, TAG_FINALIZED_STATE, None), FINALIZED.to_le_bytes().to_vec());
        Ok(())
    }

    fn encode_sub_chunk(&self, sub_chunk: &SubChunk, index: i8) -> Vec<u8> {
        let layers = sub_chunk.layers();
        let used = if layers.len() > 1 && layers[1..].iter().any(|layer| layer.count(self.air_id) != 4096) {
            layers.len()
        } else {
            1
        };
        let mut buf = vec![SUB_CHUNK_VERSION, used as u8, index as u8];
        for layer in &layers[..used] {
            layer.write_blocks_disk(&mut buf, |id, buf| {
                let nbt = self.block_nbt.get(&id).or_else(|| self.block_nbt.get(&self.air_id)).expect("air has nbt");
                buf.extend_from_slice(nbt);
            });
        }
        buf
    }

    fn heightmap(&self, chunk: &Chunk) -> [i16; 256] {
        let min_y = chunk.min_sub_chunk_y() as i32 * 16;
        let top = (chunk.highest_non_air_sub_chunk_y() as i32 + 1) * 16 - 1;
        let mut heights = [0i16; 256];
        for z in 0..16u8 {
            for x in 0..16u8 {
                let mut y = top;
                while y >= min_y && chunk.get_block(x, y, z, 0).is_none_or(|block| block == self.air_id) {
                    y -= 1;
                }
                heights[((z as usize) << 4) | x as usize] = (y + 1 - min_y) as i16;
            }
        }
        heights
    }

    pub fn load_chunk(&self, dimension: DimensionType, x: i32, z: i32) -> StorageResult<Option<Chunk>> {
        let id = dimension.id();
        if self.get(&key(id, x, z, TAG_VERSION, None))?.is_none() {
            return Ok(None);
        }

        let min = dimension.min_sub_chunk_y();
        let count = dimension.sub_chunk_count();
        let biomes = match self.get(&key(id, x, z, TAG_DATA_3D, None))? {
            Some(data) => decode_biomes(&data, count)?,
            None => vec![[PLAINS; 4096]; count],
        };

        let mut sub_chunks = Vec::with_capacity(count);
        for (offset, biomes) in biomes.iter().enumerate() {
            let index = min.wrapping_add(offset as i8);
            let sub_chunk = match self.get(&key(id, x, z, TAG_SUB_CHUNK, Some(index)))? {
                Some(data) => {
                    let layers = self.decode_sub_chunk(&data)?;
                    let air = [self.air_id; 4096];
                    SubChunk::from_layers(layers.first().unwrap_or(&air), layers.get(1), biomes, self.air_id)
                }
                None => SubChunk::from_layers(&[self.air_id; 4096], None, biomes, self.air_id),
            };
            sub_chunks.push(sub_chunk);
        }

        let mut block_entities = Vec::new();
        if let Some(data) = self.get(&key(id, x, z, TAG_BLOCK_ENTITY, None))? {
            let mut reader = data.as_slice();
            while !reader.is_empty() {
                let data: nbtx::Value = nbtx::from_le_bytes(&mut reader)?;
                let y = match &data {
                    nbtx::Value::Compound(tag) => match tag.get("y".as_bytes()) {
                        Some(nbtx::Value::Int(y)) => *y,
                        _ => continue,
                    },
                    _ => continue,
                };
                block_entities.push(BlockEntity { y, data });
            }
        }

        Ok(Some(Chunk::from_sub_chunks(x, z, min, sub_chunks, block_entities)))
    }

    fn decode_sub_chunk(&self, data: &[u8]) -> StorageResult<Vec<[i32; 4096]>> {
        let mut reader = data;
        let version = read_u8(&mut reader)?;
        let layer_count = match version {
            8 => read_u8(&mut reader)?,
            9 => {
                let count = read_u8(&mut reader)?;
                read_u8(&mut reader)?;
                count
            }
            _ => return Err(StorageError::Malformed("sub chunk version")),
        };

        let mut layers = Vec::with_capacity(layer_count as usize);
        for _ in 0..layer_count {
            let bits = (read_u8(&mut reader)? >> 1) as usize;
            let indices = read_indices(&mut reader, bits)?;
            let palette_len = read_u32(&mut reader)? as usize;
            let mut palette = Vec::with_capacity(palette_len);
            for _ in 0..palette_len {
                palette.push(self.read_block(&mut reader)?);
            }
            let mut blocks = [self.air_id; 4096];
            for (block, index) in blocks.iter_mut().zip(indices) {
                *block = *palette.get(index as usize).ok_or(StorageError::Malformed("sub chunk palette index"))?;
            }
            layers.push(blocks);
        }
        Ok(layers)
    }

    fn read_block(&self, reader: &mut &[u8]) -> StorageResult<i32> {
        let start = *reader;
        let tag: HashMap<String, nbtx::Value> = nbtx::from_le_bytes(reader)?;
        let raw = &start[..start.len() - reader.len()];

        let mut ids = self.block_ids.lock().expect("block id cache lock poisoned");
        if let Some(&id) = ids.get(raw) {
            return Ok(id);
        }

        let Some(nbtx::Value::String(name)) = tag.get("name") else {
            return Err(StorageError::Malformed("block name"));
        };
        let states = match tag.get("states") {
            Some(nbtx::Value::Compound(states)) => states.clone(),
            _ => nbtx::Compound::new(),
        };
        let id = HashUtils::hash_block_nbt(&name.to_string(), states);
        ids.insert(raw.to_vec(), id);
        Ok(id)
    }
}

fn key(dimension: i32, x: i32, z: i32, tag: u8, index: Option<i8>) -> Vec<u8> {
    let mut key = Vec::with_capacity(14);
    key.extend(x.to_le_bytes());
    key.extend(z.to_le_bytes());
    if dimension != 0 {
        key.extend(dimension.to_le_bytes());
    }
    key.push(tag);
    if let Some(index) = index {
        key.push(index as u8);
    }
    key
}

fn read_u8(reader: &mut &[u8]) -> StorageResult<u8> {
    let mut byte = [0; 1];
    reader.read_exact(&mut byte)?;
    Ok(byte[0])
}

fn read_u32(reader: &mut &[u8]) -> StorageResult<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_indices(reader: &mut &[u8], bits: usize) -> StorageResult<[u16; 4096]> {
    let mut indices = [0u16; 4096];
    if bits == 0 {
        return Ok(indices);
    }
    if bits > 16 {
        return Err(StorageError::Malformed("bits per block"));
    }
    let per_word = 32 / bits;
    let mask = (1u32 << bits) - 1;
    for word in 0..4096usize.div_ceil(per_word) {
        let packed = read_u32(reader)?;
        for slot in 0..per_word {
            let index = word * per_word + slot;
            if index < 4096 {
                indices[index] = ((packed >> (slot * bits)) & mask) as u16;
            }
        }
    }
    Ok(indices)
}

fn decode_biomes(data: &[u8], count: usize) -> StorageResult<Vec<[i32; 4096]>> {
    let mut reader = data.get(512..).ok_or(StorageError::Malformed("biome heightmap"))?;
    let mut sections: Vec<[i32; 4096]> = Vec::with_capacity(count);
    while sections.len() < count && !reader.is_empty() {
        let bits = (read_u8(&mut reader)? >> 1) as usize;
        if bits == 0x7f {
            sections.push(sections.last().copied().unwrap_or([PLAINS; 4096]));
            continue;
        }
        if bits == 0 {
            sections.push([read_u32(&mut reader)? as i32; 4096]);
            continue;
        }
        let indices = read_indices(&mut reader, bits)?;
        let palette = (0..read_u32(&mut reader)?).map(|_| read_u32(&mut reader).map(|id| id as i32)).collect::<StorageResult<Vec<_>>>()?;
        let mut biomes = [PLAINS; 4096];
        for (biome, index) in biomes.iter_mut().zip(indices) {
            *biome = *palette.get(index as usize).ok_or(StorageError::Malformed("biome palette index"))?;
        }
        sections.push(biomes);
    }
    while sections.len() < count {
        sections.push(sections.last().copied().unwrap_or([PLAINS; 4096]));
    }
    Ok(sections)
}
