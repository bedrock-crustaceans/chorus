use chorus_level::dimension_definition::DimensionDefinition;
use std::sync::Arc;

use super::biome_volume::BiomeVolume;
use super::store::BlockStore;
use crate::shared::biome::Biome;
use crate::shared::blocks::{AIR, BlockId, Blocks};
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
    store: BlockStore,
    tops: Vec<i16>,
    worldgen_heights: Option<Arc<Vec<[i16; 2]>>>,
    volume: Option<Arc<BiomeVolume>>,
    entities: Vec<PlacedEntity>,
    post_process: Vec<(u8, i16, u8)>,
    settle: Vec<(u8, i16, u8)>,
    structure_spills: Vec<(i32, i32, i32, BlockId)>,
    magma_ticks: Vec<(u8, i16, u8)>,
    fluid_ticks: Option<Arc<Vec<FluidTick>>>,
    inherited_ticks: Vec<RoutedTick>,
    routed_ticks: Vec<RoutedTick>,
    pending_fluid: Vec<RoutedTick>,
}

pub struct ChunkEdit<'a>(&'a mut ProtoChunk);

impl std::ops::Deref for ChunkEdit<'_> {
    type Target = ProtoChunk;

    fn deref(&self) -> &ProtoChunk {
        self.0
    }
}

impl std::ops::DerefMut for ChunkEdit<'_> {
    fn deref_mut(&mut self) -> &mut ProtoChunk {
        self.0
    }
}

impl Drop for ChunkEdit<'_> {
    fn drop(&mut self) {
        self.0.compact();
    }
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

#[derive(Clone, Copy, Debug)]
pub struct FluidTick {
    pub x: u8,
    pub y: i16,
    pub z: u8,
    pub lava: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoutedTick {
    pub pos: glam::IVec3,
    pub lava: bool,
    pub delay: u64,
}

impl ProtoChunk {
    pub fn with_biome_volume(x: i32, z: i32, min_y: i32, height: i32, water: BlockId, lava: BlockId, volume: BiomeVolume) -> Self {
        Self {
            x,
            z,
            min_y,
            height,
            air: AIR,
            water,
            lava,
            store: BlockStore::new(min_y, height, AIR),
            tops: vec![min_y as i16; 256],
            worldgen_heights: None,
            volume: Some(Arc::new(volume)),
            entities: Vec::new(),
            post_process: Vec::new(),
            settle: Vec::new(),
            structure_spills: Vec::new(),
            magma_ticks: Vec::new(),
            fluid_ticks: None,
            inherited_ticks: Vec::new(),
            routed_ticks: Vec::new(),
            pending_fluid: Vec::new(),
        }
    }

    pub fn empty(definition: DimensionDefinition) -> Self {
        Self {
            x: 0,
            z: 0,
            min_y: definition.min_y,
            height: definition.height,
            air: AIR,
            water: AIR,
            lava: AIR,
            store: BlockStore::new(definition.min_y, definition.height, AIR),
            tops: Vec::new(),
            worldgen_heights: None,
            volume: None,
            entities: Vec::new(),
            post_process: Vec::new(),
            settle: Vec::new(),
            structure_spills: Vec::new(),
            magma_ticks: Vec::new(),
            fluid_ticks: None,
            inherited_ticks: Vec::new(),
            routed_ticks: Vec::new(),
            pending_fluid: Vec::new(),
        }
    }

    pub fn empty_beside(&self, dx: i32, dz: i32) -> Self {
        Self {
            x: self.x + dx,
            z: self.z + dz,
            min_y: self.min_y,
            height: self.height,
            air: self.air,
            water: self.water,
            lava: self.lava,
            store: BlockStore::new(self.min_y, self.height, self.air),
            tops: if self.tops.is_empty() { Vec::new() } else { vec![self.min_y as i16; 256] },
            worldgen_heights: None,
            volume: self.volume.clone(),
            entities: Vec::new(),
            post_process: Vec::new(),
            settle: Vec::new(),
            structure_spills: Vec::new(),
            magma_ticks: Vec::new(),
            fluid_ticks: None,
            inherited_ticks: Vec::new(),
            routed_ticks: Vec::new(),
            pending_fluid: Vec::new(),
        }
    }

    pub fn min_x(&self) -> i32 {
        self.x << 4
    }

    pub fn min_z(&self) -> i32 {
        self.z << 4
    }

    pub fn definition(&self) -> DimensionDefinition {
        DimensionDefinition {
            min_y: self.min_y,
            height: self.height,
        }
    }

    pub fn max_y(&self) -> i32 {
        self.min_y + self.height - 1
    }

    pub fn edit(&mut self) -> ChunkEdit<'_> {
        ChunkEdit(self)
    }

    pub fn compact(&mut self) {
        self.store.compact();
    }

    pub fn fill_empty(&mut self, blocks: &[BlockId]) {
        self.store.fill_empty(blocks);
    }

    pub fn read_cells(&self, out: &mut [BlockId]) {
        self.store.read_cells(out);
    }

    pub fn write_cells(&mut self, blocks: &[BlockId]) {
        self.store.write_cells(blocks);
    }

    pub fn set_tops(&mut self, tops: &[i16; 256]) {
        self.tops.copy_from_slice(tops);
    }

    pub fn contains_y(&self, y: i32) -> bool {
        (self.min_y..self.min_y + self.height).contains(&y)
    }

    pub fn cells(&self) -> usize {
        256 * self.height as usize
    }

    pub fn index(&self, x: i32, y: i32, z: i32) -> usize {
        ((x * 16 + z) as usize) * self.height as usize + (y - self.min_y) as usize
    }

    pub fn get_at(&self, index: usize) -> BlockId {
        let (x, y, z) = self.coordinates(index);
        self.store.get(x, y, z)
    }

    pub fn set_at(&mut self, index: usize, block: BlockId) {
        let (x, y, z) = self.coordinates(index);
        self.set(x, y, z, block);
    }

    pub fn cells_vec(&self) -> Vec<BlockId> {
        self.store.columns_vec()
    }

    pub fn column_tops_below(&self, limit: i32) -> [i32; 256] {
        self.store.column_tops_below(limit)
    }

    pub fn section_palette(&self, section: usize) -> Option<&[i32]> {
        self.store.section_palette(section)
    }

    fn coordinates(&self, index: usize) -> (i32, i32, i32) {
        let height = self.height as usize;
        let column = index / height;
        ((column / 16) as i32, self.min_y + (index % height) as i32, (column % 16) as i32)
    }

    pub fn discard_dense(&mut self) {
        self.store.discard_dense();
    }

    fn section_array(&self, section: usize) -> [i32; 4096] {
        self.store.section_array(section)
    }

    fn top_block_section(&self) -> usize {
        self.store.top_block_section()
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.store.get(x, y, z)
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        self.store.set(x, y, z, block);
        if !self.tops.is_empty() && self.contains_y(y) {
            self.track_top(x, y, z, block);
        }
    }

    fn track_top(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        let column = ((z << 4) | x) as usize;
        if block != self.air {
            self.tops[column] = self.tops[column].max(y as i16 + 1);
        } else if y + 1 == self.tops[column] as i32 {
            let mut top = y;
            while top > self.min_y && self.get(x, top - 1, z) == self.air {
                top -= 1;
            }
            self.tops[column] = top as i16;
        }
    }

    pub fn freeze_worldgen_heights(&mut self, blocks: &Blocks) {
        let heights = (0..256)
            .map(|column| {
                let (x, z) = (column as i32 & 15, column as i32 >> 4);
                let surface = (self.min_y..self.tops[column] as i32).rev().find(|&y| !blocks.is_air(self.get(x, y, z))).map_or(self.min_y, |y| y + 1);
                let floor = (self.min_y..surface).rev().find(|&y| blocks.entry(self.get(x, y, z)).solid).map_or(self.min_y, |y| y + 1);
                [surface as i16, floor as i16]
            })
            .collect();
        self.worldgen_heights = Some(Arc::new(heights));
    }

    pub fn worldgen_height(&self, ocean_floor: bool, x: i32, z: i32) -> Option<i32> {
        self.worldgen_heights.as_ref()?.get(((z << 4) | x) as usize).map(|heights| heights[ocean_floor as usize] as i32)
    }

    pub fn top(&self, x: i32, z: i32) -> i32 {
        self.tops[((z << 4) | x) as usize] as i32
    }

    pub fn highest_block_y(&self) -> Option<i32> {
        self.tops.iter().max().map(|&top| top as i32 - 1)
    }

    pub fn mark_post_process(&mut self, x: u8, y: i16, z: u8) {
        if !self.post_process.contains(&(x, y, z)) {
            self.post_process.push((x, y, z));
        }
    }

    pub fn add_structure_spills(&mut self, spills: impl IntoIterator<Item = (i32, i32, i32, BlockId)>) {
        self.structure_spills.extend(spills);
    }

    pub fn set_fluid_ticks(&mut self, ticks: Vec<FluidTick>) {
        self.fluid_ticks = Some(Arc::new(ticks));
    }

    pub fn set_inherited_ticks(&mut self, ticks: Vec<RoutedTick>) {
        self.inherited_ticks = ticks;
    }

    pub fn inherited_ticks(&self) -> &[RoutedTick] {
        &self.inherited_ticks
    }

    pub fn set_routed_ticks(&mut self, ticks: Vec<RoutedTick>) {
        self.routed_ticks = ticks;
    }

    pub fn routed_ticks(&self) -> &[RoutedTick] {
        &self.routed_ticks
    }

    pub fn set_pending_fluid(&mut self, ticks: Vec<RoutedTick>) {
        self.pending_fluid = ticks;
    }

    pub fn pending_fluid(&self) -> &[RoutedTick] {
        &self.pending_fluid
    }

    pub fn take_pending_fluid(&mut self) -> Vec<RoutedTick> {
        std::mem::take(&mut self.pending_fluid)
    }

    pub fn fluid_ticks(&self) -> &[FluidTick] {
        self.fluid_ticks.as_deref().map_or(&[], Vec::as_slice)
    }

    pub fn structure_spills(&self) -> &[(i32, i32, i32, BlockId)] {
        &self.structure_spills
    }

    pub fn mark_magma_tick(&mut self, x: u8, y: i16, z: u8) {
        if !self.magma_ticks.contains(&(x, y, z)) {
            self.magma_ticks.push((x, y, z));
        }
    }

    pub fn magma_ticks(&self) -> &[(u8, i16, u8)] {
        &self.magma_ticks
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

    fn volume(&self) -> &BiomeVolume {
        self.volume.as_deref().expect("a chunk generated with biomes")
    }

    pub fn biomes(&self) -> impl Iterator<Item = Biome> + '_ {
        (0..self.height / 4).flat_map(move |qy| (0..4).flat_map(move |qz| (0..4).map(move |qx| self.volume().get(qx << 2, self.min_y + (qy << 2), qz << 2))))
    }

    pub fn biome_at(&self, x: i32, y: i32, z: i32) -> Biome {
        self.volume().get(x, y, z)
    }

    pub fn biome_id_at(&self, x: i32, y: i32, z: i32) -> i32 {
        self.volume().id_at(x, y, z)
    }

    pub fn uniform_biome_at(&self, y: i32) -> Option<Biome> {
        self.volume().uniform_biome_at(y)
    }

    pub fn biome_palette_len_at(&self, y: i32) -> usize {
        self.volume().palette_len_at(y)
    }

    pub fn biome_section_ids_at(&self, y: i32) -> [i32; 4096] {
        self.volume().section_ids_at(y)
    }

    pub fn uniform_biome_id_at(&self, y: i32) -> Option<i32> {
        self.volume().uniform_id_at(y)
    }

    pub fn into_chunk(self, blocks: &Blocks) -> Chunk {
        let air = blocks.runtime(AIR);
        let water = blocks.runtime(self.water);
        let top_block_section = self.top_block_section();
        let build = |section: usize| {
            let mut layer = [air; 4096];
            let mut extra = [air; 4096];
            let mut waterlogged = false;
            let mut biomes = [0; 4096];
            let section_blocks = self.section_array(section).map(|block| block as BlockId);
            let section_biomes = self.volume().section_ids(section.min(top_block_section));
            for (local, (&block, &biome)) in section_blocks.iter().zip(&section_biomes).enumerate() {
                let index = SubChunk::index((local & 15) as u8, (local >> 8) as u8, ((local >> 4) & 15) as u8);
                let entry = blocks.entry(block);
                layer[index] = entry.runtime;
                if entry.flooded {
                    extra[index] = water;
                    waterlogged = true;
                }
                biomes[index] = biome;
            }
            SubChunk::from_layers(&layer, waterlogged.then_some(&extra), &biomes, air)
        };
        let sections = (self.height / 16) as usize;
        let mut sub_chunks = Vec::with_capacity(sections);
        let mut sky: Option<SubChunk> = None;
        for section in 0..sections {
            if section > top_block_section {
                if let Some(sky) = &sky {
                    sub_chunks.push(sky.clone());
                    continue;
                }
                let built = build(section);
                sky = Some(built.clone());
                sub_chunks.push(built);
            } else {
                sub_chunks.push(build(section));
            }
        }
        let entities = self
            .entities
            .iter()
            .filter(|entity| self.get(entity.x as i32, entity.y as i32, entity.z as i32) == entity.block)
            .map(|entity| ChunkBlockEntity {
                y: entity.y as i32,
                data: entity.to_nbt(self.x, self.z),
            })
            .collect();
        let updates = self
            .pending_fluid
            .iter()
            .map(|tick| chorus_level::fluid::PendingUpdate {
                pos: tick.pos,
                lava: tick.lava,
                delay: tick.delay,
            })
            .collect();
        let mut chunk = Chunk::from_sub_chunks(self.x, self.z, (self.min_y >> 4) as i8, sub_chunks, entities);
        chunk.set_fluid_updates(updates);
        chunk
    }
}

pub struct QuartBiomes {
    biomes: Vec<i32>,
    surface_levels: [i32; 16],
}

impl QuartBiomes {
    pub fn new(biomes: Vec<i32>, surface_levels: [i32; 16]) -> Self {
        Self { biomes, surface_levels }
    }

    pub fn get(&self, qx: i32, qy: i32, qz: i32) -> Biome {
        let id = self.biomes[((qy + 1) * 16 + qz * 4 + qx) as usize];
        Biome::generated(id)
    }

    pub fn raw_samples(&self) -> &[i32] {
        &self.biomes
    }

    pub fn surface_levels(&self, chunk_x: i32, chunk_z: i32) -> impl Iterator<Item = ((i32, i32), i32)> + '_ {
        self.surface_levels
            .iter()
            .enumerate()
            .map(move |(i, &level)| (((chunk_x << 4) + (i as i32 % 4) * 4, (chunk_z << 4) + (i as i32 / 4) * 4), level))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NETHER: DimensionDefinition = DimensionDefinition::NETHER;

    #[test]
    fn indexed_access_matches_coordinate_access() {
        let mut chunk = ProtoChunk::empty(NETHER);
        chunk.set(3, 77, 12, 9);
        assert_eq!(chunk.get_at(chunk.index(3, 77, 12)), 9);
        chunk.set_at(chunk.index(15, 0, 15), 4);
        assert_eq!(chunk.get(15, 0, 15), 4);

        let mut chunk = ProtoChunk::empty(NETHER);
        chunk.set(1, 2, 3, 7);
        let cells = chunk.cells_vec();
        assert_eq!(cells.len(), chunk.cells());
        assert_eq!(cells[chunk.index(1, 2, 3)], 7);
        assert_eq!(cells.iter().filter(|&&block| block != AIR).count(), 1);
    }

    #[test]
    fn a_bare_chunk_tracks_no_tops_and_holds_no_side_tables() {
        let chunk = ProtoChunk::empty(NETHER);
        assert!(chunk.tops.is_empty());
        assert!(chunk.volume.is_none());
        assert!(chunk.worldgen_heights.is_none());
        assert!(chunk.fluid_ticks.is_none());
        assert_eq!(chunk.entities.capacity() + chunk.post_process.capacity() + chunk.routed_ticks.capacity(), 0);
    }
}
