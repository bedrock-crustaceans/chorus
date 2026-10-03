use super::biome::Biome::{self, *};
use super::blocks::{BlockId, BlockTable};
use super::proto::ProtoChunk;
use super::random::{PositionalFactory, RandomSource, Xoroshiro};
use super::terrain::{self, MIN_Y, Terrain};
use crate::block::block_id::*;

const WAY_BELOW_MIN_Y: i32 = -2032 << 4;
const COPPER_VEIN: (i32, i32) = (0, 50);
const IRON_VEIN: (i32, i32) = (-60, -8);

pub struct SurfaceBlocks {
    air: BlockId,
    pub bedrock: BlockId,
    stone: BlockId,
    deepslate: BlockId,
    pub dirt: BlockId,
    podzol: BlockId,
    coarse_dirt: BlockId,
    pub mycelium: BlockId,
    pub grass_block: BlockId,
    calcite: BlockId,
    gravel: BlockId,
    sand: BlockId,
    sandstone: BlockId,
    red_sand: BlockId,
    red_sandstone: BlockId,
    packed_ice: BlockId,
    snow_block: BlockId,
    mud: BlockId,
    powder_snow: BlockId,
    ice: BlockId,
    water: BlockId,
    terracotta: BlockId,
    orange_terracotta: BlockId,
    white_terracotta: BlockId,
    cinnabar: BlockId,
    sulfur: BlockId,
    copper_ore: BlockId,
    raw_copper_block: BlockId,
    granite: BlockId,
    deepslate_iron_ore: BlockId,
    raw_iron_block: BlockId,
    tuff: BlockId,
}

impl SurfaceBlocks {
    pub fn new(blocks: &mut BlockTable) -> Self {
        let mut b = |name: &'static str| blocks.name(name);
        Self {
            air: b(AIR),
            bedrock: b(BEDROCK),
            stone: b(STONE),
            deepslate: b(DEEPSLATE),
            dirt: b(DIRT),
            podzol: b(PODZOL),
            coarse_dirt: b(COARSE_DIRT),
            mycelium: b(MYCELIUM),
            grass_block: b(GRASS_BLOCK),
            calcite: b(CALCITE),
            gravel: b(GRAVEL),
            sand: b(SAND),
            sandstone: b(SANDSTONE),
            red_sand: b(RED_SAND),
            red_sandstone: b(RED_SANDSTONE),
            packed_ice: b(PACKED_ICE),
            snow_block: b(SNOW),
            mud: b(MUD),
            powder_snow: b(POWDER_SNOW),
            ice: b(ICE),
            water: b(WATER),
            terracotta: b(HARDENED_CLAY),
            orange_terracotta: b(ORANGE_TERRACOTTA),
            white_terracotta: b(WHITE_TERRACOTTA),
            cinnabar: b(CINNABAR),
            sulfur: b(SULFUR),
            copper_ore: b(COPPER_ORE),
            raw_copper_block: b(RAW_COPPER_BLOCK),
            granite: b(GRANITE),
            deepslate_iron_ore: b(DEEPSLATE_IRON_ORE),
            raw_iron_block: b(RAW_IRON_BLOCK),
            tuff: b(TUFF),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Floor {
    Gradient,
    Columns,
}

pub struct MaterialSystem {
    pub blocks: SurfaceBlocks,
    sea_level: i32,
    floor: Floor,
    random: PositionalFactory,
    bedrock_floor: PositionalFactory,
    deepslate: PositionalFactory,
    ore: PositionalFactory,
    clay_bands: Vec<BlockId>,
}

impl MaterialSystem {
    pub fn new(blocks: &mut BlockTable, random: PositionalFactory, sea_level: i32, floor: Floor) -> Self {
        let colors = [ORANGE_TERRACOTTA, YELLOW_TERRACOTTA, BROWN_TERRACOTTA, RED_TERRACOTTA, WHITE_TERRACOTTA, LIGHT_GRAY_TERRACOTTA].map(|color| blocks.name(color));
        let terracotta = blocks.name(HARDENED_CLAY);
        Self {
            blocks: SurfaceBlocks::new(blocks),
            sea_level,
            floor,
            random,
            bedrock_floor: match floor {
                Floor::Gradient => random.hash_of("minecraft:bedrock_floor").fork_positional(),
                Floor::Columns => Xoroshiro::new(0).fork_positional().hash_of("minecraft:bedrock_floor").fork_positional(),
            },
            deepslate: random.hash_of("minecraft:deepslate").fork_positional(),
            ore: random.hash_of("minecraft:ore").fork_positional(),
            clay_bands: generate_bands(&mut random.hash_of("minecraft:clay_bands"), terracotta, colors),
        }
    }

    fn surface_depth(&self, terrain: &Terrain, x: i32, z: i32) -> i32 {
        let noise = terrain.surface.surface.get(x as f64, 0.0, z as f64) as f64;
        (noise * 2.75 + 3.0 + self.random.at(x, 0, z).next_double() * 0.25) as i32
    }

    fn band(&self, terrain: &Terrain, x: i32, y: i32, z: i32) -> BlockId {
        let offset = (terrain.surface.clay_bands_offset.get(x as f64, 0.0, z as f64) * 4.0).round() as i32;
        let length = self.clay_bands.len() as i32;
        self.clay_bands[(y + offset + length).rem_euclid(length) as usize]
    }

    pub fn surface_corners(terrain: &Terrain, chunk: &ProtoChunk) -> [f32; 4] {
        [(0, 0), (16, 0), (0, 16), (16, 16)].map(|(dx, dz)| terrain.preliminary_surface_level(chunk.min_x() + dx, chunk.min_z() + dz))
    }

    fn chunk_surface_level(corners: [f32; 4]) -> impl Fn(i32, i32) -> f32 {
        move |x: i32, z: i32| {
            let (ax, az) = (x as f32 / 16.0, z as f32 / 16.0);
            super::noise::lerp2(ax, az, corners[0], corners[1], corners[2], corners[3])
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn top_material(&self, terrain: &Terrain, noise: &terrain::NoiseChunk, chunk: &ProtoChunk, corners: [f32; 4], x: i32, y: i32, z: i32, under_fluid: bool) -> Option<BlockId> {
        let chunk_surface_level = Self::chunk_surface_level(corners);
        let height_at = |x: i32, z: i32| chunk.surface_height(x.clamp(0, 15), z.clamp(0, 15));
        let (block_x, block_z) = (chunk.min_x() + x, chunk.min_z() + z);
        let surface_depth = self.surface_depth(terrain, block_x, block_z);
        let mut context = Context {
            system: self,
            terrain,
            x: block_x,
            y,
            z: block_z,
            local_x: x,
            local_z: z,
            gradient_x: height_at(x + 1, z) - height_at(x - 1, z),
            gradient_z: height_at(x, z + 1) - height_at(x, z - 1),
            surface_depth,
            surface_secondary: None,
            surface_noise: None,
            min_surface_level: chunk_surface_level(x, z).floor() as i32 + surface_depth - 8,
            stone_depth_above: 1,
            stone_depth_below: 1,
            water_height: if under_fluid { y + 1 } else { i32::MIN },
            biome: chunk.source_biome_at(x, y, z),
            noise,
        };
        context.overworld()
    }

    pub fn build(&self, terrain: &Terrain, noise: &terrain::NoiseChunk, chunk: &mut ProtoChunk) {
        let min_y = chunk.min_y;
        let chunk_surface_level = Self::chunk_surface_level(Self::surface_corners(terrain, chunk));

        let heights: Vec<i32> = (0..256).map(|i| chunk.surface_height(i % 16, i / 16)).collect();
        let height_at = |x: i32, z: i32| heights[(z.clamp(0, 15) * 16 + x.clamp(0, 15)) as usize];

        for x in 0..16 {
            for z in 0..16 {
                let block_x = chunk.min_x() + x;
                let block_z = chunk.min_z() + z;
                let starting_height = height_at(x, z) + 1;
                let surface_biome = chunk.biome_at(x, starting_height, z);
                if surface_biome == ErodedBadlands {
                    self.eroded_badlands_extension(terrain, chunk, x, z, block_x, block_z, starting_height);
                }
                let height = chunk.surface_height(x, z) + 1;
                let surface_depth = self.surface_depth(terrain, block_x, block_z);
                let mut state = Context {
                    system: self,
                    terrain,
                    x: block_x,
                    y: 0,
                    z: block_z,
                    local_x: x,
                    local_z: z,
                    gradient_x: height_at(x + 1, z) - height_at(x - 1, z),
                    gradient_z: height_at(x, z + 1) - height_at(x, z - 1),
                    surface_depth,
                    surface_secondary: None,
                    surface_noise: None,
                    min_surface_level: chunk_surface_level(x, z).floor() as i32 + surface_depth - 8,
                    stone_depth_above: 0,
                    stone_depth_below: 0,
                    water_height: i32::MIN,
                    biome: surface_biome,
                    noise,
                };

                let mut stone_depth_above = 0;
                let mut water_height = i32::MIN;
                let mut next_ceiling_stone_y = i32::MAX;
                let mut y = height;
                while y >= min_y {
                    let old = chunk.get(x, y, z);
                    if old == chunk.air {
                        stone_depth_above = 0;
                        water_height = i32::MIN;
                    } else if chunk.is_fluid(old) {
                        if water_height == i32::MIN {
                            water_height = y + 1;
                        }
                    } else {
                        if next_ceiling_stone_y >= y {
                            next_ceiling_stone_y = WAY_BELOW_MIN_Y;
                            let mut lookahead = y - 1;
                            while lookahead >= min_y - 1 {
                                let next = chunk.get(x, lookahead, z);
                                if next == chunk.air || chunk.is_fluid(next) {
                                    next_ceiling_stone_y = lookahead + 1;
                                    break;
                                }
                                lookahead -= 1;
                            }
                        }
                        stone_depth_above += 1;
                        state.y = y;
                        state.stone_depth_above = stone_depth_above;
                        state.stone_depth_below = y - next_ceiling_stone_y + 1;
                        state.water_height = water_height;
                        state.biome = chunk.biome_at(x, y, z);
                        if let Some(block) = state.overworld() {
                            chunk.set(x, y, z, block);
                        }
                    }
                    y -= 1;
                }

                if matches!(surface_biome, FrozenOcean | DeepFrozenOcean) {
                    let min_surface = state.min_surface_level;
                    self.frozen_ocean_extension(terrain, chunk, x, z, block_x, block_z, starting_height, min_surface, surface_biome);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn eroded_badlands_extension(&self, terrain: &Terrain, chunk: &mut ProtoChunk, x: i32, z: i32, bx: i32, bz: i32, height: i32) {
        let noises = &terrain.surface;
        let (fx, fz) = (bx as f64, bz as f64);
        let pillar_buffer = (noises.badlands_surface.get(fx, 0.0, fz) as f64 * 8.25)
            .abs()
            .min((noises.badlands_pillar.get(fx * 0.2, 0.0, fz * 0.2) * 15.0) as f64);
        if pillar_buffer <= 0.0 {
            return;
        }
        let pillar_floor = (noises.badlands_pillar_roof.get(fx * 0.75, 0.0, fz * 0.75) as f64 * 1.5).abs();
        let top = 64.0 + (pillar_buffer * pillar_buffer * 2.5).min((pillar_floor * 50.0).ceil() + 24.0);
        let start_y = top.floor() as i32;
        if height > start_y {
            return;
        }
        let mut y = start_y;
        while y >= chunk.min_y {
            let old = chunk.get(x, y, z);
            if old == self.blocks.stone {
                break;
            }
            if old == chunk.water {
                return;
            }
            y -= 1;
        }
        let mut y = start_y;
        while y >= chunk.min_y && chunk.get(x, y, z) == chunk.air {
            chunk.set(x, y, z, self.blocks.stone);
            y -= 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn frozen_ocean_extension(&self, terrain: &Terrain, chunk: &mut ProtoChunk, x: i32, z: i32, bx: i32, bz: i32, height: i32, min_surface: i32, biome: Biome) {
        let noises = &terrain.surface;
        let (fx, fz) = (bx as f64, bz as f64);
        let iceberg = (noises.iceberg_surface.get(fx, 0.0, fz) as f64 * 8.25)
            .abs()
            .min((noises.iceberg_pillar.get(fx * 1.28, 0.0, fz * 1.28) * 15.0) as f64);
        if iceberg <= 1.8 {
            return;
        }
        let roof = (noises.iceberg_pillar_roof.get(fx * 1.17, 0.0, fz * 1.17) as f64 * 1.5).abs();
        let mut top = (iceberg * iceberg * 1.2).min((roof * 40.0).ceil() + 14.0);
        if biome.temperature_at(bx, self.sea_level, bz, self.sea_level) > 0.1 {
            top -= 2.0;
        }
        if top <= 2.0 {
            return;
        }
        let bottom = self.sea_level as f64 - top - 7.0;
        let extension_top = top + self.sea_level as f64;
        let mut random = self.random.at(bx, 0, bz);
        let max_snow_depth = 2 + random.next_int_bounded(4);
        let min_snow_height = self.sea_level + 18 + random.next_int_bounded(10);
        let mut snow_depth = 0;
        let mut y = height.max(extension_top as i32 + 1);
        while y >= min_surface {
            let block = chunk.get(x, y, z);
            if (block == chunk.air && y < extension_top as i32 && random.next_double() > 0.01) || (block == chunk.water && y > bottom as i32 && y < self.sea_level && random.next_double() > 0.15) {
                if snow_depth <= max_snow_depth && y > min_snow_height {
                    chunk.set(x, y, z, self.blocks.snow_block);
                    snow_depth += 1;
                } else {
                    chunk.set(x, y, z, self.blocks.packed_ice);
                }
            }
            y -= 1;
        }
    }
}

fn generate_bands(random: &mut Xoroshiro, terracotta: BlockId, [orange, yellow, brown, red, white, light_gray]: [BlockId; 6]) -> Vec<BlockId> {
    let mut bands = vec![terracotta; 192];
    let mut i = 0;
    while i < bands.len() {
        i += random.next_int_bounded(5) as usize + 1;
        if i < bands.len() {
            bands[i] = orange;
        }
        i += 1;
    }
    for (base_width, block) in [(1, yellow), (2, brown), (1, red)] {
        let count = random.next_int_between_inclusive(6, 15);
        for _ in 0..count {
            let width = base_width + random.next_int_bounded(3) as usize;
            let start = random.next_int_bounded(bands.len() as i32) as usize;
            let mut p = 0;
            while start + p < bands.len() && p < width {
                bands[start + p] = block;
                p += 1;
            }
        }
    }
    let white_count = random.next_int_between_inclusive(9, 15);
    let mut placed = 0;
    let mut start = 0;
    while placed < white_count && start < bands.len() {
        bands[start] = white;
        if start > 1 && random.next_boolean() {
            bands[start - 1] = light_gray;
        }
        if start + 1 < bands.len() && random.next_boolean() {
            bands[start + 1] = light_gray;
        }
        placed += 1;
        start += random.next_int_bounded(16) as usize + 4;
    }
    bands
}

struct Context<'a> {
    system: &'a MaterialSystem,
    terrain: &'a Terrain,
    x: i32,
    y: i32,
    z: i32,
    local_x: i32,
    local_z: i32,
    gradient_x: i32,
    gradient_z: i32,
    surface_depth: i32,
    surface_secondary: Option<f64>,
    surface_noise: Option<f64>,
    min_surface_level: i32,
    stone_depth_above: i32,
    stone_depth_below: i32,
    water_height: i32,
    biome: Biome,
    noise: &'a terrain::NoiseChunk,
}

impl Context<'_> {
    fn is(&self, biomes: &[Biome]) -> bool {
        biomes.contains(&self.biome)
    }

    fn bedrock_floor(&self) -> bool {
        match self.system.floor {
            Floor::Gradient => self.vertical_gradient(&self.system.bedrock_floor, MIN_Y, MIN_Y + 5),
            Floor::Columns => {
                let height = 1 + (self.system.bedrock_floor.at(self.x, MIN_Y, self.z).next_float() * 4.0) as i32;
                self.y <= MIN_Y + height.min(4)
            }
        }
    }

    fn vertical_gradient(&self, random: &PositionalFactory, true_at_and_below: i32, false_at_and_above: i32) -> bool {
        if self.y <= true_at_and_below {
            return true;
        }
        if self.y >= false_at_and_above {
            return false;
        }
        let probability = 1.0 - (self.y - true_at_and_below) as f64 / (false_at_and_above - true_at_and_below) as f64;
        (random.at(self.x, self.y, self.z).next_float() as f64) < probability
    }

    fn surface_secondary(&mut self) -> f64 {
        *self
            .surface_secondary
            .get_or_insert_with(|| self.terrain.surface.surface_secondary.get(self.x as f64, 0.0, self.z as f64) as f64)
    }

    fn surface_noise(&mut self) -> f64 {
        *self.surface_noise.get_or_insert_with(|| self.terrain.surface.surface.get(self.x as f64, 0.0, self.z as f64) as f64)
    }

    fn noise_2d(&self, noise: &super::noise::NoiseStack, min: f64, max: f64) -> bool {
        let value = noise.get(self.x as f64, 0.0, self.z as f64) as f64;
        value >= min && value <= max
    }

    fn surface_noise_above(&mut self, threshold: f64) -> bool {
        self.surface_noise() >= threshold / 8.25
    }

    fn stone_depth(&mut self, add_surface_depth: bool, secondary_depth_range: i32, ceiling: bool) -> bool {
        let stone_depth = if ceiling { self.stone_depth_below } else { self.stone_depth_above };
        let surface_depth = if add_surface_depth { self.surface_depth } else { 0 };
        let secondary = if secondary_depth_range == 0 {
            0
        } else {
            ((self.surface_secondary() + 1.0) / 2.0 * secondary_depth_range as f64) as i32
        };
        stone_depth <= 1 + surface_depth + secondary
    }

    fn on_floor(&mut self) -> bool {
        self.stone_depth(false, 0, false)
    }

    fn under_floor(&mut self) -> bool {
        self.stone_depth(true, 0, false)
    }

    fn on_ceiling(&mut self) -> bool {
        self.stone_depth(false, 0, true)
    }

    fn water(&self, offset: i32, multiplier: i32, add_stone_depth: bool) -> bool {
        self.water_height == i32::MIN || self.y + if add_stone_depth { self.stone_depth_above } else { 0 } >= self.water_height + offset + self.surface_depth * multiplier
    }

    fn not_underwater(&self) -> bool {
        self.water(-1, 0, false)
    }

    fn not_under_deep_water(&self) -> bool {
        self.water(-6, -1, true)
    }

    fn y_block(&self, anchor: i32, multiplier: i32) -> bool {
        self.y >= anchor + self.surface_depth * multiplier
    }

    fn y_start(&self, anchor: i32, multiplier: i32) -> bool {
        self.y + self.stone_depth_above >= anchor + self.surface_depth * multiplier
    }

    fn steep(&self) -> bool {
        self.gradient_x <= -4 || self.gradient_z >= 4
    }

    fn hole(&self) -> bool {
        self.surface_depth <= 0
    }

    fn overworld(&mut self) -> Option<BlockId> {
        let system = self.system;
        let b = &system.blocks;
        if self.bedrock_floor() {
            return Some(b.bedrock);
        }
        if let Some(block) = self.ore_vein(COPPER_VEIN, true, b.copper_ore, b.raw_copper_block, b.granite) {
            return Some(block);
        }
        if let Some(block) = self.ore_vein(IRON_VEIN, false, b.deepslate_iron_ore, b.raw_iron_block, b.tuff) {
            return Some(block);
        }
        if self.y >= self.min_surface_level
            && let Some(block) = self.surface()
        {
            return Some(block);
        }
        self.underground()
    }

    fn ore_vein(&mut self, (min_y, max_y): (i32, i32), toggle_positive: bool, ore: BlockId, raw_ore: BlockId, filler: BlockId) -> Option<BlockId> {
        if self.y < min_y || self.y >= max_y {
            return None;
        }
        let vein = self.noise.vein(self.local_x, self.y, self.local_z);
        let density = vein.density(self.y, min_y, max_y, toggle_positive);
        if density <= 0.0 {
            return None;
        }
        let mut random = self.system.ore.at(self.x, self.y, self.z);
        if random.next_float() > density {
            return None;
        }
        if random.next_float() < vein.richness() && self.terrain.ore_gap(self.x, self.y, self.z) < 0.0 {
            Some(if random.next_float() < 0.02 { raw_ore } else { ore })
        } else {
            Some(filler)
        }
    }

    fn underground(&mut self) -> Option<BlockId> {
        if self.biome == SulfurCaves
            && let Some(block) = self.sulfur_cave_bands()
        {
            return Some(block);
        }
        if self.vertical_gradient(&self.system.deepslate, 0, 8) {
            return Some(self.system.blocks.deepslate);
        }
        None
    }

    fn sulfur_cave_bands(&self) -> Option<BlockId> {
        let value = self.terrain.surface.sulfur_cave_gradient.get(self.x as f64, self.y as f64, self.z as f64) as f64;
        let system = self.system;
        let b = &system.blocks;
        if (-0.4f32 as f64..=-0.1f32 as f64).contains(&value) {
            Some(b.cinnabar)
        } else if (0.0..=0.4f32 as f64).contains(&value) {
            Some(b.sulfur)
        } else if value >= 0.4f32 as f64 {
            Some(b.cinnabar)
        } else {
            None
        }
    }

    fn sand_or_sandstone_if_ceiling(&mut self) -> BlockId {
        if self.on_ceiling() { self.system.blocks.sandstone } else { self.system.blocks.sand }
    }

    fn gravel_or_stone_if_ceiling(&mut self) -> BlockId {
        if self.on_ceiling() { self.system.blocks.stone } else { self.system.blocks.gravel }
    }

    fn default_biome_surface(&self) -> BlockId {
        if self.not_underwater() { self.system.blocks.grass_block } else { self.system.blocks.dirt }
    }

    fn clay_band(&mut self) -> bool {
        let value = self.surface_noise();
        (-0.909..=-0.5454).contains(&value) || (-0.1818..=0.1818).contains(&value) || (0.5454..=0.909).contains(&value)
    }

    fn common_surface_and_under(&mut self) -> Option<BlockId> {
        let system = self.system;
        let b = &system.blocks;
        let terrain = self.terrain;
        let noises = &terrain.surface;
        match self.biome {
            StonyPeaks => Some(if self.noise_2d(&noises.calcite, -0.0125, 0.0125) { b.calcite } else { b.stone }),
            StonyShore => Some(if self.noise_2d(&noises.gravel, -0.05, 0.05) { self.gravel_or_stone_if_ceiling() } else { b.stone }),
            WindsweptHills => self.surface_noise_above(1.0).then_some(b.stone),
            WarmOcean | Beach | SnowyBeach | Desert => Some(self.sand_or_sandstone_if_ceiling()),
            DripstoneCaves => Some(b.stone),
            SulfurCaves => Some(self.sulfur_cave_bands().unwrap_or(b.stone)),
            MangroveSwamp => Some(b.mud),
            _ => None,
        }
    }

    fn powder_snow(&self, min: f64, max: f64) -> Option<BlockId> {
        (self.noise_2d(&self.terrain.surface.powder_snow, min, max) && self.not_underwater()).then_some(self.system.blocks.powder_snow)
    }

    fn under_biome_surface(&mut self) -> Option<BlockId> {
        let system = self.system;
        let b = &system.blocks;
        let terrain = self.terrain;
        let noises = &terrain.surface;
        match self.biome {
            FrozenPeaks => {
                if self.steep() || self.noise_2d(&noises.packed_ice, -0.5, 0.2) {
                    return Some(b.packed_ice);
                }
                if self.noise_2d(&noises.ice, -0.0625, 0.025) {
                    return Some(b.ice);
                }
                if self.not_underwater() {
                    return Some(b.snow_block);
                }
            }
            SnowySlopes => {
                if self.steep() {
                    return Some(b.stone);
                }
                if let Some(block) = self.powder_snow(0.45, 0.58) {
                    return Some(block);
                }
                if self.not_underwater() {
                    return Some(b.snow_block);
                }
            }
            JaggedPeaks => return Some(b.stone),
            Grove => return Some(self.powder_snow(0.45, 0.58).unwrap_or(b.dirt)),
            _ => {}
        }
        if let Some(block) = self.common_surface_and_under() {
            return Some(block);
        }
        match self.biome {
            WindsweptSavanna => {
                if self.surface_noise_above(1.75) {
                    return Some(b.stone);
                }
            }
            WindsweptGravellyHills => {
                if self.surface_noise_above(2.0) {
                    return Some(self.gravel_or_stone_if_ceiling());
                }
                if self.surface_noise_above(1.0) {
                    return Some(b.stone);
                }
                if self.surface_noise_above(-1.0) {
                    return Some(b.dirt);
                }
                return Some(self.gravel_or_stone_if_ceiling());
            }
            _ => {}
        }
        Some(b.dirt)
    }

    fn biome_surface(&mut self) -> Option<BlockId> {
        let system = self.system;
        let b = &system.blocks;
        let terrain = self.terrain;
        let noises = &terrain.surface;
        match self.biome {
            FrozenPeaks => {
                if self.steep() || self.noise_2d(&noises.packed_ice, 0.0, 0.2) {
                    return Some(b.packed_ice);
                }
                if self.noise_2d(&noises.ice, 0.0, 0.025) {
                    return Some(b.ice);
                }
                if self.not_underwater() {
                    return Some(b.snow_block);
                }
            }
            SnowySlopes => {
                if self.steep() {
                    return Some(b.stone);
                }
                if let Some(block) = self.powder_snow(0.35, 0.6) {
                    return Some(block);
                }
                if self.not_underwater() {
                    return Some(b.snow_block);
                }
            }
            JaggedPeaks => {
                if self.steep() {
                    return Some(b.stone);
                }
                if self.not_underwater() {
                    return Some(b.snow_block);
                }
            }
            Grove => {
                if let Some(block) = self.powder_snow(0.35, 0.6) {
                    return Some(block);
                }
                if self.not_underwater() {
                    return Some(b.snow_block);
                }
            }
            _ => {}
        }
        if let Some(block) = self.common_surface_and_under() {
            return Some(block);
        }
        match self.biome {
            WindsweptSavanna => {
                if self.surface_noise_above(1.75) {
                    return Some(b.stone);
                }
                if self.surface_noise_above(-0.5) {
                    return Some(b.coarse_dirt);
                }
            }
            WindsweptGravellyHills => {
                if self.surface_noise_above(2.0) {
                    return Some(self.gravel_or_stone_if_ceiling());
                }
                if self.surface_noise_above(1.0) {
                    return Some(b.stone);
                }
                if self.surface_noise_above(-1.0) {
                    return Some(self.default_biome_surface());
                }
                return Some(self.gravel_or_stone_if_ceiling());
            }
            OldGrowthPineTaiga | OldGrowthSpruceTaiga => {
                if self.surface_noise_above(1.75) {
                    return Some(b.coarse_dirt);
                }
                if self.surface_noise_above(-0.95) {
                    return Some(b.podzol);
                }
            }
            IceSpikes => {
                if self.not_underwater() {
                    return Some(b.snow_block);
                }
            }
            MushroomFields => return Some(b.mycelium),
            DappledForest if self.noise_2d(&noises.surface_patch_small, 1.2f32 as f64, f64::MAX) => return Some(b.coarse_dirt),
            _ => {}
        }
        Some(self.default_biome_surface())
    }

    fn surface(&mut self) -> Option<BlockId> {
        let system = self.system;
        let b = &system.blocks;
        let terrain = self.terrain;
        let noises = &terrain.surface;
        if self.on_floor() {
            match self.biome {
                WoodedBadlands if self.y_block(97, 2) => return Some(if self.clay_band() { b.coarse_dirt } else { self.default_biome_surface() }),
                Swamp if self.y_block(62, 0) && !self.y_block(63, 0) && self.noise_2d(&noises.swamp, 0.0, f64::MAX) => return Some(b.water),
                MangroveSwamp if self.y_block(60, 0) && !self.y_block(63, 0) && self.noise_2d(&noises.swamp, 0.0, f64::MAX) => return Some(b.water),
                _ => {}
            }
        }
        if self.is(&[Badlands, ErodedBadlands, WoodedBadlands]) {
            if self.on_floor() {
                if self.y_block(256, 0) {
                    return Some(b.orange_terracotta);
                }
                if self.y_start(74, 1) {
                    return Some(if self.clay_band() { b.terracotta } else { self.system.band(self.terrain, self.x, self.y, self.z) });
                }
                if self.not_underwater() {
                    return Some(if self.on_ceiling() { b.red_sandstone } else { b.red_sand });
                }
                if !self.hole() {
                    return Some(b.orange_terracotta);
                }
                if self.not_under_deep_water() {
                    return Some(b.white_terracotta);
                }
                return Some(self.gravel_or_stone_if_ceiling());
            }
            if self.y_start(63, -1) {
                if self.y_block(63, 0) && !self.y_start(74, 1) {
                    return Some(b.orange_terracotta);
                }
                return Some(self.system.band(self.terrain, self.x, self.y, self.z));
            }
            if self.under_floor() && self.not_under_deep_water() {
                return Some(b.white_terracotta);
            }
        }
        let frozen_ocean = self.is(&[FrozenOcean, DeepFrozenOcean]);
        if self.on_floor() && self.not_underwater() {
            if frozen_ocean && self.hole() {
                return Some(b.air);
            }
            if let Some(block) = self.biome_surface() {
                return Some(block);
            }
        }
        if self.not_under_deep_water() {
            if self.on_floor() && frozen_ocean && self.hole() {
                return Some(b.water);
            }
            if self.under_floor()
                && let Some(block) = self.under_biome_surface()
            {
                return Some(block);
            }
            if self.is(&[WarmOcean, Beach, SnowyBeach]) && self.stone_depth(true, 6, false) {
                return Some(b.sandstone);
            }
            if self.biome == Desert && self.stone_depth(true, 30, false) {
                return Some(b.sandstone);
            }
        }
        if self.on_floor() {
            if self.is(&[FrozenPeaks, JaggedPeaks]) {
                return Some(b.stone);
            }
            if self.is(&[WarmOcean, LukewarmOcean, DeepLukewarmOcean]) {
                return Some(self.sand_or_sandstone_if_ceiling());
            }
            return Some(self.gravel_or_stone_if_ceiling());
        }
        None
    }
}
