use std::sync::Arc;

use super::biome::Biome;
use super::blocks::{AIR, BlockId, Blocks};
use chorus_level::chunk::{BlockEntity as ChunkBlockEntity, Chunk};
use chorus_level::sub_chunk::SubChunk;

#[derive(Clone)]
pub struct ProtoChunk {
    pub x: i32,
    pub z: i32,
    pub min_y: i32,
    pub height: i32,
    pub air: BlockId,
    pub water: BlockId,
    pub lava: BlockId,
    blocks: Vec<BlockId>,
    tops: Vec<i32>,
    worldgen_heights: Arc<Vec<[i32; 2]>>,
    biomes: Vec<Biome>,
    fiddles: Arc<Vec<[f64; 3]>>,
    zoom: Arc<Vec<Biome>>,
    zoom_seed: Option<i64>,
    entities: Vec<PlacedEntity>,
    post_process: Vec<(u8, i16, u8)>,
    settle: Vec<(u8, i16, u8)>,
}

#[derive(Clone, Debug)]
pub enum BlockEntity {
    Chest { loot_table: &'static str, seed: i64 },
    Spawner { entity: &'static str, width: f32, height: f32 },
    Beehive,
}

#[derive(Clone, Debug)]
pub struct PlacedEntity {
    pub x: u8,
    pub y: i16,
    pub z: u8,
    pub block: BlockId,
    pub entity: BlockEntity,
}

impl PlacedEntity {
    fn to_nbt(&self, chunk_x: i32, chunk_z: i32) -> nbtx::Value {
        use nbtx::Value;
        let mut tag = nbtx::Compound::new();
        let mut put = |key: &str, value: Value| {
            tag.insert(key.into(), value);
        };
        let id = match self.entity {
            BlockEntity::Chest { .. } => "Chest",
            BlockEntity::Spawner { .. } => "MobSpawner",
            BlockEntity::Beehive => "Beehive",
        };
        put("id", Value::String(id.into()));
        put("x", Value::Int((chunk_x << 4) + self.x as i32));
        put("y", Value::Int(self.y as i32));
        put("z", Value::Int((chunk_z << 4) + self.z as i32));
        put("isMovable", Value::Byte(1));
        match self.entity {
            BlockEntity::Chest { loot_table, seed } => {
                put("Findable", Value::Byte(0));
                put("LootTable", Value::String(format!("loot_tables/chests/{loot_table}.json").into()));
                put("LootTableSeed", Value::Int(seed as i32));
            }
            BlockEntity::Spawner { entity, width, height } => {
                put("EntityIdentifier", Value::String(entity.into()));
                put("Delay", Value::Short(20));
                put("MinSpawnDelay", Value::Short(200));
                put("MaxSpawnDelay", Value::Short(800));
                put("SpawnCount", Value::Short(4));
                put("MaxNearbyEntities", Value::Short(6));
                put("RequiredPlayerRange", Value::Short(16));
                put("SpawnRange", Value::Short(4));
                put("DisplayEntityWidth", Value::Float(width));
                put("DisplayEntityHeight", Value::Float(height));
                put("DisplayEntityScale", Value::Float(1.0));
            }
            BlockEntity::Beehive => put("ShouldSpawnBees", Value::Byte(1)),
        }
        Value::Compound(tag)
    }
}

const QUART_SPAN: i32 = 6;

impl ProtoChunk {
    #[allow(clippy::too_many_arguments)]
    pub fn new(x: i32, z: i32, min_y: i32, height: i32, water: BlockId, lava: BlockId, zoom_seed: Option<i64>, around: [&QuartBiomes; 9]) -> Self {
        let levels = height / 4 + 2;
        let mut fiddles = Vec::with_capacity((QUART_SPAN * QUART_SPAN * levels) as usize);
        for qy in (-1..=height / 4).filter(|_| zoom_seed.is_some()) {
            for qz in -1..=4 {
                for qx in -1..=4 {
                    fiddles.push(corner_fiddles(zoom_seed.unwrap_or_default(), ((x << 2) + qx, (min_y >> 2) + qy, (z << 2) + qz)));
                }
            }
        }
        let mut chunk = Self {
            x,
            z,
            min_y,
            height,
            air: AIR,
            water,
            lava,
            blocks: vec![AIR; (height * 256) as usize],
            tops: vec![min_y; 256],
            worldgen_heights: Arc::new(Vec::new()),
            biomes: vec![Biome::Plains; (QUART_SPAN * QUART_SPAN * levels) as usize],
            fiddles: Arc::new(fiddles),
            zoom: Arc::new(Vec::new()),
            zoom_seed,
            entities: Vec::new(),
            post_process: Vec::new(),
            settle: Vec::new(),
        };
        chunk.assemble_biomes(around);
        chunk.zoom = Arc::new(if zoom_seed.is_some() { chunk.compute_zoom() } else { chunk.compute_plain() });
        chunk
    }

    pub fn min_x(&self) -> i32 {
        self.x << 4
    }

    pub fn min_z(&self) -> i32 {
        self.z << 4
    }

    pub fn max_y(&self) -> i32 {
        self.min_y + self.height - 1
    }

    fn index(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        let ry = y - self.min_y;
        (0..self.height).contains(&ry).then_some(((ry << 8) | (z << 4) | x) as usize)
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.index(x, y, z).map_or(self.air, |index| self.blocks[index])
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        let Some(index) = self.index(x, y, z) else { return };
        self.blocks[index] = block;
        let column = ((z << 4) | x) as usize;
        if block != self.air {
            self.tops[column] = self.tops[column].max(y + 1);
        } else if y + 1 == self.tops[column] {
            let mut top = y;
            while top > self.min_y && self.get(x, top - 1, z) == self.air {
                top -= 1;
            }
            self.tops[column] = top;
        }
    }

    pub fn freeze_worldgen_heights(&mut self, blocks: &Blocks) {
        let heights = (0..256)
            .map(|column| {
                let (x, z) = (column as i32 & 15, column as i32 >> 4);
                let surface = (self.min_y..self.tops[column]).rev().find(|&y| !blocks.is_air(self.get(x, y, z))).map_or(self.min_y, |y| y + 1);
                let floor = (self.min_y..surface).rev().find(|&y| blocks.entry(self.get(x, y, z)).solid).map_or(self.min_y, |y| y + 1);
                [surface, floor]
            })
            .collect();
        self.worldgen_heights = Arc::new(heights);
    }

    pub fn worldgen_height(&self, ocean_floor: bool, x: i32, z: i32) -> Option<i32> {
        self.worldgen_heights.get(((z << 4) | x) as usize).map(|heights| heights[ocean_floor as usize])
    }

    pub fn top(&self, x: i32, z: i32) -> i32 {
        self.tops[((z << 4) | x) as usize]
    }

    pub fn mark_post_process(&mut self, x: u8, y: i16, z: u8) {
        if !self.post_process.contains(&(x, y, z)) {
            self.post_process.push((x, y, z));
        }
    }

    pub fn post_process(&self) -> &[(u8, i16, u8)] {
        &self.post_process
    }

    pub fn mark_settle(&mut self, x: u8, y: i16, z: u8) {
        self.settle.push((x, y, z));
    }

    pub fn take_settle(&mut self) -> Vec<(u8, i16, u8)> {
        let mut settle = std::mem::take(&mut self.settle);
        settle.sort_unstable();
        settle.dedup();
        settle
    }

    pub fn add_entity(&mut self, entity: PlacedEntity) {
        self.entities.retain(|existing| (existing.x, existing.y, existing.z) != (entity.x, entity.y, entity.z));
        self.entities.push(entity);
    }

    pub fn is_fluid(&self, block: BlockId) -> bool {
        block == self.water || block == self.lava
    }

    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        self.top(x, z) - 1
    }

    fn quart_index(&self, qx: i32, qy: i32, qz: i32) -> usize {
        (((qy + 1) * QUART_SPAN + qz + 1) * QUART_SPAN + qx + 1) as usize
    }

    fn assemble_biomes(&mut self, around: [&QuartBiomes; 9]) {
        for qy in -1..=self.height / 4 {
            for qz in -1i32..=4 {
                for qx in -1i32..=4 {
                    let (dx, dz) = (qx.div_euclid(4), qz.div_euclid(4));
                    let source = around[((dx + 1) * 3 + dz + 1) as usize];
                    let index = self.quart_index(qx, qy, qz);
                    self.biomes[index] = source.get(qx.rem_euclid(4), qy, qz.rem_euclid(4));
                }
            }
        }
    }

    pub fn biomes(&self) -> impl Iterator<Item = Biome> + '_ {
        (0..self.height / 4).flat_map(move |qy| (0..4).flat_map(move |qz| (0..4).map(move |qx| self.biomes[self.quart_index(qx, qy, qz)])))
    }

    pub fn biome_at(&self, x: i32, y: i32, z: i32) -> Biome {
        match self.index(x, y, z) {
            Some(index) => self.zoom[index],
            None => self.zoomed(x, y, z, true),
        }
    }

    fn plain(&self, x: i32, y: i32, z: i32, clamp: bool) -> Biome {
        let mut qy = (y >> 2) - (self.min_y >> 2);
        if clamp {
            qy = qy.clamp(0, self.height / 4 - 1);
        }
        self.biomes[self.quart_index(x >> 2, qy, z >> 2)]
    }

    fn compute_plain(&self) -> Vec<Biome> {
        let mut plain = vec![Biome::Plains; (self.height * 256) as usize];
        for y in self.min_y..=self.max_y() {
            for z in 0..16 {
                for x in 0..16 {
                    plain[(((y - self.min_y) << 8) | (z << 4) | x) as usize] = self.plain(x, y, z, true);
                }
            }
        }
        plain
    }

    fn compute_zoom(&self) -> Vec<Biome> {
        let mut zoom = vec![Biome::Plains; (self.height * 256) as usize];
        let parent_min_y = (self.min_y - 2) >> 2;
        let parent_max_y = (self.max_y() - 2) >> 2;
        let local_y = |parent: i32| parent - (self.min_y >> 2);
        for py in parent_min_y..=parent_max_y {
            for pz in -1..=3 {
                for px in -1..=3 {
                    let candidates: [Biome; 8] = std::array::from_fn(|corner| {
                        let qy = (local_y(py) + (corner & 2 != 0) as i32).clamp(0, self.height / 4 - 1);
                        self.biomes[self.quart_index(px + (corner & 4 != 0) as i32, qy, pz + (corner & 1 != 0) as i32)]
                    });
                    if candidates.iter().all(|&biome| biome == candidates[0]) {
                        for y in ((py << 2) + 2).max(self.min_y)..((py << 2) + 6).min(self.max_y() + 1) {
                            for z in ((pz << 2) + 2).max(0)..((pz << 2) + 6).min(16) {
                                for x in ((px << 2) + 2).max(0)..((px << 2) + 6).min(16) {
                                    zoom[(((y - self.min_y) << 8) | (z << 4) | x) as usize] = candidates[0];
                                }
                            }
                        }
                        continue;
                    }
                    let mut squares = [[[0.0f64; 4]; 3]; 8];
                    for (i, corner_squares) in squares.iter_mut().enumerate() {
                        let odd = [(i & 4 != 0) as i32, (i & 2 != 0) as i32, (i & 1 != 0) as i32];
                        let fiddle = self.fiddles[self.quart_index(px + odd[0], local_y(py) + odd[1], pz + odd[2])];
                        for (axis, axis_squares) in corner_squares.iter_mut().enumerate() {
                            for (step, square) in axis_squares.iter_mut().enumerate() {
                                let distance = step as f64 / 4.0 - odd[axis] as f64;
                                *square = (distance + fiddle[axis]).powi(2);
                            }
                        }
                    }
                    for fy in 0..4 {
                        let y = (py << 2) + fy + 2;
                        if y < self.min_y || y > self.max_y() {
                            continue;
                        }
                        for fz in 0..4 {
                            let z = (pz << 2) + fz + 2;
                            if !(0..16).contains(&z) {
                                continue;
                            }
                            for fx in 0..4 {
                                let x = (px << 2) + fx + 2;
                                if !(0..16).contains(&x) {
                                    continue;
                                }
                                let mut best = 0;
                                let mut best_distance = f64::INFINITY;
                                for (i, corner) in squares.iter().enumerate() {
                                    let distance = corner[2][fz as usize] + corner[1][fy as usize] + corner[0][fx as usize];
                                    if best_distance > distance {
                                        best = i;
                                        best_distance = distance;
                                    }
                                }
                                zoom[(((y - self.min_y) << 8) | (z << 4) | x) as usize] = candidates[best];
                            }
                        }
                    }
                }
            }
        }
        zoom
    }

    #[cfg(test)]
    pub fn zoom_matches_direct(&self) -> bool {
        (self.min_y..=self.max_y()).all(|y| (0..16).all(|z| (0..16).all(|x| self.biome_at(x, y, z) == self.zoomed(x, y, z, true))))
    }

    pub fn source_biome_at(&self, x: i32, y: i32, z: i32) -> Biome {
        self.zoomed(x, y, z, false)
    }

    fn zoomed(&self, x: i32, y: i32, z: i32, clamp: bool) -> Biome {
        let Some(zoom_seed) = self.zoom_seed else { return self.plain(x, y, z, clamp) };
        let (abs_x, abs_y, abs_z) = (self.min_x() + x - 2, y - 2, self.min_z() + z - 2);
        let parent = (abs_x >> 2, abs_y >> 2, abs_z >> 2);
        let fract = ((abs_x & 3) as f64 / 4.0, (abs_y & 3) as f64 / 4.0, (abs_z & 3) as f64 / 4.0);
        let mut best = 0;
        let mut best_distance = f64::INFINITY;
        for i in 0..8 {
            let (x_even, y_even, z_even) = (i & 4 == 0, i & 2 == 0, i & 1 == 0);
            let corner = (parent.0 + !x_even as i32, parent.1 + !y_even as i32, parent.2 + !z_even as i32);
            let distance = (
                if x_even { fract.0 } else { fract.0 - 1.0 },
                if y_even { fract.1 } else { fract.1 - 1.0 },
                if z_even { fract.2 } else { fract.2 - 1.0 },
            );
            let local = (corner.0 - (self.x << 2), corner.1 - (self.min_y >> 2), corner.2 - (self.z << 2));
            let [fiddle_x, fiddle_y, fiddle_z] = if (-1..=self.height / 4).contains(&local.1) {
                self.fiddles[self.quart_index(local.0, local.1, local.2)]
            } else {
                corner_fiddles(zoom_seed, corner)
            };
            let fiddled = (distance.2 + fiddle_z).powi(2) + (distance.1 + fiddle_y).powi(2) + (distance.0 + fiddle_x).powi(2);
            if best_distance > fiddled {
                best = i;
                best_distance = fiddled;
            }
        }
        let qx = parent.0 + (best & 4 != 0) as i32 - (self.x << 2);
        let qz = parent.2 + (best & 1 != 0) as i32 - (self.z << 2);
        let mut qy = parent.1 + (best & 2 != 0) as i32 - (self.min_y >> 2);
        if clamp {
            qy = qy.clamp(0, self.height / 4 - 1);
        }
        self.biomes[self.quart_index(qx, qy, qz)]
    }

    pub fn into_chunk(self, blocks: &Blocks) -> Chunk {
        let air = blocks.runtime(AIR);
        let water = blocks.runtime(self.water);
        let sub_chunks = (0..self.height / 16)
            .map(|section| {
                let mut layer = [air; 4096];
                let mut extra = [air; 4096];
                let mut waterlogged = false;
                let mut biomes = [0; 4096];
                let start = (section as usize) << 12;
                let section_blocks = &self.blocks[start..start + 4096];
                let section_biomes = &self.zoom[start..start + 4096];
                for (local, (&block, &biome)) in section_blocks.iter().zip(section_biomes).enumerate() {
                    let index = SubChunk::index((local & 15) as u8, (local >> 8) as u8, ((local >> 4) & 15) as u8);
                    let entry = blocks.entry(block);
                    layer[index] = entry.runtime;
                    if entry.flooded {
                        extra[index] = water;
                        waterlogged = true;
                    }
                    biomes[index] = biome.bedrock_id();
                }
                SubChunk::from_layers(&layer, waterlogged.then_some(&extra), &biomes, air)
            })
            .collect();
        let entities = self
            .entities
            .iter()
            .filter(|entity| self.get(entity.x as i32, entity.y as i32, entity.z as i32) == entity.block)
            .map(|entity| ChunkBlockEntity {
                y: entity.y as i32,
                data: entity.to_nbt(self.x, self.z),
            })
            .collect();
        Chunk::from_sub_chunks(self.x, self.z, (self.min_y >> 4) as i8, sub_chunks, entities)
    }
}

pub fn zoom_seed(seed: i64) -> i64 {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(seed.to_le_bytes());
    i64::from_le_bytes(hash[..8].try_into().expect("sha256 is 32 bytes"))
}

fn lcg(value: i64, salt: i64) -> i64 {
    value.wrapping_mul(value.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)).wrapping_add(salt)
}

fn fiddle(value: i64) -> f64 {
    ((value >> 24).rem_euclid(1024) as f64 / 1024.0 - 0.5) * 0.9
}

fn corner_fiddles(seed: i64, (x, y, z): (i32, i32, i32)) -> [f64; 3] {
    let mut value = seed;
    for salt in [x, y, z, x, y, z] {
        value = lcg(value, salt as i64);
    }
    let fiddle_x = fiddle(value);
    value = lcg(value, seed);
    let fiddle_y = fiddle(value);
    value = lcg(value, seed);
    [fiddle_x, fiddle_y, fiddle(value)]
}

pub struct QuartBiomes {
    biomes: Vec<Biome>,
    surface_levels: [i32; 16],
}

impl QuartBiomes {
    pub fn new(biomes: Vec<Biome>, surface_levels: [i32; 16]) -> Self {
        Self { biomes, surface_levels }
    }

    pub fn get(&self, qx: i32, qy: i32, qz: i32) -> Biome {
        self.biomes[((qy + 1) * 16 + qz * 4 + qx) as usize]
    }

    pub fn surface_levels(&self, chunk_x: i32, chunk_z: i32) -> impl Iterator<Item = ((i32, i32), i32)> + '_ {
        self.surface_levels
            .iter()
            .enumerate()
            .map(move |(i, &level)| (((chunk_x << 4) + (i as i32 % 4) * 4, (chunk_z << 4) + (i as i32 / 4) * 4), level))
    }
}
