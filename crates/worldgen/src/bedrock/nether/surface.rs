use super::biome::BIOMES;
use super::terrain::{NetherNoises, SEA_LEVEL, TERRAIN_HEIGHT};
use super::{NetherBlocks, NetherChunk};
use crate::shared::biome::BiomeRegistry;
use crate::shared::biome_documents::ROOT;
use crate::shared::blocks::{AIR, BlockId, BlockTable};
use crate::shared::chunk::store::{give_scratch, take_scratch};
use crate::shared::noise::legacy::LegacyPerlin;
use crate::shared::rng::mersenne::MersenneTwister;
use serde_json::Value;

const DEPTH_FREQUENCY: f32 = 0.0625;
const BEACH_DEPTH: i32 = 5;
const SINGLE_OCTAVE: usize = 1;
const SURFACE_BUILDER: &str = "minecraft:surface_builder";
const MATERIAL_ADJUSTMENTS: &str = "minecraft:surface_material_adjustments";
const OVERWORLD_BUILDER: &str = "minecraft:overworld";
const CAPPED_BUILDER: &str = "minecraft:capped";
const SEA_LEVEL_VARIABLE: &str = "variable.sea_level";
const DEFAULT_NOISE_SCALE: f32 = 1.0;
const WHOLE_COLUMN: [i32; 2] = [0, TERRAIN_HEIGHT - 1];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Materials {
    top: BlockId,
    mid: BlockId,
    sea_floor: BlockId,
    foundation: BlockId,
    sea: BlockId,
}

#[derive(Debug, PartialEq)]
struct Adjustment {
    noise_scale: f32,
    range: [f32; 2],
    heights: [i32; 2],
    top: Option<BlockId>,
    mid: Option<BlockId>,
    sea_floor: Option<BlockId>,
    foundation: Option<BlockId>,
}

struct Capped {
    floor: Vec<BlockId>,
    ceiling: Vec<BlockId>,
    beach: Option<BlockId>,
    sea: BlockId,
    foundation: BlockId,
    noises: CappedNoises,
}

enum Builder {
    Default {
        sea_floor_depth: i32,
        materials: Materials,
        adjustments: Vec<Adjustment>,
    },
    Capped(Capped),
}

fn block(table: &mut BlockTable, value: Option<&'static Value>) -> Option<BlockId> {
    value.and_then(Value::as_str).map(|name| table.name(name))
}

fn required(table: &mut BlockTable, builder: &'static Value, key: &str) -> BlockId {
    block(table, builder.get(key)).unwrap_or_else(|| panic!("a nether surface builder has no {key}"))
}

fn blocks(table: &mut BlockTable, value: Option<&'static Value>) -> Vec<BlockId> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|name| name.as_str())
        .map(|name| table.name(name))
        .collect()
}

fn height(value: &Value) -> i32 {
    if let Some(number) = value.as_i64() {
        return number as i32;
    }
    let text: String = value
        .as_str()
        .unwrap_or_else(|| panic!("a height range entry is neither a number nor a string"))
        .split_whitespace()
        .collect();
    match text.strip_prefix(SEA_LEVEL_VARIABLE) {
        Some("") => SEA_LEVEL,
        Some(offset) => {
            SEA_LEVEL
                + offset
                    .parse::<i32>()
                    .unwrap_or_else(|_| panic!("the height range {text} is not {SEA_LEVEL_VARIABLE} plus or minus a number"))
        }
        None => text
            .parse()
            .unwrap_or_else(|_| panic!("the height range {text} is not a number or {SEA_LEVEL_VARIABLE} plus or minus a number")),
    }
}

fn parse_adjustments(table: &mut BlockTable, components: &'static Value) -> Vec<Adjustment> {
    let entries = components.get(MATERIAL_ADJUSTMENTS).and_then(|adjustments| adjustments["adjustments"].as_array());
    entries
        .into_iter()
        .flatten()
        .map(|entry| {
            let materials = &entry["materials"];
            let noise = |index: usize| entry["noise_range"][index].as_f64().expect("a noise range bound") as f32;
            Adjustment {
                noise_scale: entry.get("noise_frequency_scale").and_then(Value::as_f64).map_or(DEFAULT_NOISE_SCALE, |scale| scale as f32),
                range: [noise(0), noise(1)],
                heights: entry.get("height_range").map_or(WHOLE_COLUMN, |range| [height(&range[0]), height(&range[1])]),
                top: block(table, materials.get("top_material")),
                mid: block(table, materials.get("mid_material")),
                sea_floor: block(table, materials.get("sea_floor_material")),
                foundation: block(table, materials.get("foundation_material")),
            }
        })
        .collect()
}

fn parse_builder(table: &mut BlockTable, seed: u32, components: &'static Value) -> Option<Builder> {
    let builder = &components[SURFACE_BUILDER]["builder"];
    match builder["type"].as_str()? {
        OVERWORLD_BUILDER => Some(Builder::Default {
            sea_floor_depth: builder["sea_floor_depth"].as_i64().expect("a sea floor depth") as i32,
            materials: Materials {
                top: required(table, builder, "top_material"),
                mid: required(table, builder, "mid_material"),
                sea_floor: required(table, builder, "sea_floor_material"),
                foundation: required(table, builder, "foundation_material"),
                sea: required(table, builder, "sea_material"),
            },
            adjustments: parse_adjustments(table, components),
        }),
        CAPPED_BUILDER => {
            let floor = blocks(table, builder.get("floor_materials"));
            let ceiling = blocks(table, builder.get("ceiling_materials"));
            let noises = CappedNoises::new(seed, floor.len(), ceiling.len());
            Some(Builder::Capped(Capped {
                floor,
                ceiling,
                beach: block(table, builder.get("beach_material")),
                sea: required(table, builder, "sea_material"),
                foundation: required(table, builder, "foundation_material"),
                noises,
            }))
        }
        _ => None,
    }
}

struct CappedNoises {
    floor: Vec<LegacyPerlin>,
    ceiling: Vec<LegacyPerlin>,
}

impl CappedNoises {
    fn new(seed: u32, floor_count: usize, ceiling_count: usize) -> Self {
        let noise = |index: usize| LegacyPerlin::new(&mut MersenneTwister::new(seed.wrapping_add(index as u32)), SINGLE_OCTAVE);
        let floor = (0..floor_count).map(noise).collect();
        let ceiling = (floor_count..floor_count + ceiling_count).map(noise).collect();
        Self { floor, ceiling }
    }
}

pub struct Context<'a> {
    pub noises: &'a NetherNoises,
    pub ids: &'a NetherBlocks,
}

pub struct NetherSurface {
    builders: Vec<(i32, Builder)>,
}

impl NetherSurface {
    pub fn new(seed: i64, table: &mut BlockTable) -> Self {
        let root: &'static Value = &ROOT;
        let registry = BiomeRegistry::vanilla();
        let builders = BIOMES
            .iter()
            .filter_map(|&biome| {
                let pack = registry.pack_name(biome)?;
                let components = &root.get(pack)?["components"];
                parse_builder(table, seed as u32, components).map(|builder| (biome, builder))
            })
            .collect();
        Self { builders }
    }

    fn builder(&self, biome: i32) -> Option<&Builder> {
        self.builders.iter().find(|(known, _)| *known == biome).map(|(_, builder)| builder)
    }

    pub fn build(&self, proto: &mut NetherChunk, context: &Context, chunk_x: i32, chunk_z: i32, column_biomes: &[i32; 256], random: &mut MersenneTwister) {
        let mut blocks = take_scratch(proto.cells(), AIR);
        proto.read_cells(&mut blocks);
        let mut cells = Cells {
            blocks: &mut blocks,
            min_y: proto.min_y,
        };
        let chunk = &mut cells;
        for x in 0..16 {
            for z in 0..16 {
                let world = (chunk_x * 16 + x, chunk_z * 16 + z);
                let biome = column_biomes[(x * 16 + z) as usize];
                let depth_noise = context.noises.depth_noise(world.0, world.1, DEPTH_FREQUENCY);
                match self.builder(biome) {
                    Some(Builder::Default {
                        sea_floor_depth,
                        materials,
                        adjustments,
                    }) => {
                        let kept: Vec<&Adjustment> = adjustments
                            .iter()
                            .filter(|adjustment| {
                                let value = context.noises.adjustment_noise(world.0, world.1, adjustment.noise_scale);
                                adjustment.range[0] <= value && value < adjustment.range[1]
                            })
                            .collect();
                        let depth = surface_depth(random, depth_noise);
                        let at = |y: i32| adjusted(*materials, &kept, y);
                        build_default(chunk, (x, z), at, *sea_floor_depth, depth, random, context.ids);
                    }
                    Some(Builder::Capped(capped)) => {
                        build_capped(chunk, (x, z), world, capped, depth_noise, random, context.ids);
                    }
                    None => {}
                }
            }
        }
        proto.write_cells(&blocks);
        give_scratch(blocks);
    }
}

struct Cells<'a> {
    blocks: &'a mut [BlockId],
    min_y: i32,
}

impl Cells<'_> {
    fn index(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        let row = usize::try_from(y - self.min_y).ok()?;
        let index = (row << 8) | ((z << 4) | x) as usize;
        (index < self.blocks.len()).then_some(index)
    }

    fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.index(x, y, z).map_or(AIR, |index| self.blocks[index])
    }

    fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        if let Some(index) = self.index(x, y, z) {
            self.blocks[index] = block;
        }
    }
}

fn adjusted(materials: Materials, kept: &[&Adjustment], y: i32) -> Materials {
    let mut result = materials;
    for adjustment in kept.iter().filter(|adjustment| adjustment.heights[0] <= y && y <= adjustment.heights[1]) {
        result = Materials {
            top: adjustment.top.unwrap_or(result.top),
            mid: adjustment.mid.unwrap_or(result.mid),
            sea_floor: adjustment.sea_floor.unwrap_or(result.sea_floor),
            foundation: adjustment.foundation.unwrap_or(result.foundation),
            sea: result.sea,
        };
    }
    result
}

fn surface_depth(random: &mut MersenneTwister, noise: f32) -> i32 {
    (random.next_float() * 0.25 + (noise / 3.0 + 3.0)) as i32
}

fn build_default(chunk: &mut Cells, (x, z): (i32, i32), materials: impl Fn(i32) -> Materials, sea_floor_depth: i32, depth: i32, random: &mut MersenneTwister, ids: &NetherBlocks) {
    let sea = SEA_LEVEL;
    let sea_floor_limit = sea_floor_depth + depth;
    let start = materials(TERRAIN_HEIGHT - 1);
    let (mut top_material, mut under_material) = (start.top, start.mid);
    let mut run = -1;
    let mut y = TERRAIN_HEIGHT - 1;
    while y > 0 {
        let block = chunk.get(x, y, z);
        let at = materials(y);
        if block == ids.bedrock {
        } else if block == ids.air {
            run = -1;
        } else if block == ids.stone {
            if run == -1 {
                let (mut top, under) = if depth <= 0 {
                    (ids.air, at.foundation)
                } else if y >= sea - 4 && y <= sea + 1 {
                    (at.top, at.mid)
                } else {
                    (top_material, under_material)
                };
                if y < sea && top == ids.air {
                    top = at.sea;
                }
                run = depth;
                if y < sea - 1 {
                    if y == 1 || y < sea - sea_floor_limit {
                        chunk.set(x, y, z, at.sea_floor);
                        (top_material, under_material) = (ids.air, at.foundation);
                        y -= 1;
                        continue;
                    }
                    chunk.set(x, y, z, under);
                } else {
                    chunk.set(x, y, z, top);
                }
                (top_material, under_material) = (top, under);
            } else if run == 0 {
                chunk.set(x, y, z, at.foundation);
            } else {
                chunk.set(x, y, z, under_material);
                run -= 1;
                let _ = random;
            }
        } else if block == ids.water || block == ids.lava {
            chunk.set(x, y, z, at.sea);
        }
        y -= 1;
    }
}

fn pick(materials: &[BlockId], noises: &[LegacyPerlin], at: [f32; 3]) -> BlockId {
    if materials.len() == 1 {
        return materials[0];
    }
    let mut best = (materials[0], -1.0f32);
    for (&material, noise) in materials.iter().zip(noises) {
        let value = noise.value(at) * noise.normalisation;
        if best.1 < value {
            best = (material, value);
        }
    }
    best.0
}

fn solid(block: u16, ids: &NetherBlocks) -> bool {
    block != ids.air && block != ids.water && block != ids.lava
}

fn build_capped(chunk: &mut Cells, (x, z): (i32, i32), world: (i32, i32), capped: &Capped, depth_noise: f32, random: &mut MersenneTwister, ids: &NetherBlocks) {
    let noises = &capped.noises;
    let floor_depth = surface_depth(random, depth_noise);
    let ceiling_depth = surface_depth(random, depth_noise);
    let at = [world.0 as f32 * 0.0625, SEA_LEVEL as f32, world.1 as f32 * 0.0625];
    let floor = pick(&capped.floor, &noises.floor, at);
    let ceiling = pick(&capped.ceiling, &noises.ceiling, at);
    let beach_noise = &noises.floor[0];
    let beach = capped
        .beach
        .filter(|_| beach_noise.value([world.0 as f32 * 0.03125, 109.0, world.1 as f32 * 0.03125]) * beach_noise.normalisation > 0.0);
    let mut below = solid(chunk.get(x, 0, z), ids);
    for y in 0..TERRAIN_HEIGHT {
        let here = solid(chunk.get(x, y, z), ids);
        if here != below {
            if !here {
                fill(
                    chunk,
                    (x, z),
                    y - 1,
                    (y - floor_depth - 1).clamp(-1, TERRAIN_HEIGHT),
                    |cell| match beach {
                        Some(beach) if (SEA_LEVEL - BEACH_DEPTH..=SEA_LEVEL).contains(&cell) => beach,
                        _ => floor,
                    },
                    ids,
                );
            } else {
                fill(chunk, (x, z), y, (y + ceiling_depth).clamp(-1, TERRAIN_HEIGHT), |_| ceiling, ids);
            }
            below = here;
        }
        let block = chunk.get(x, y, z);
        if block == ids.stone {
            chunk.set(x, y, z, capped.foundation);
        } else if block == ids.water {
            chunk.set(x, y, z, capped.sea);
        }
    }
}

fn fill(chunk: &mut Cells, (x, z): (i32, i32), from: i32, until: i32, material: impl Fn(i32) -> u16, ids: &NetherBlocks) {
    let step = if from <= until { 1 } else { -1 };
    let mut cell = from;
    while cell != until && (0..TERRAIN_HEIGHT).contains(&cell) && solid(chunk.get(x, cell, z), ids) {
        chunk.set(x, cell, z, material(cell));
        cell += step;
    }
}

const FLOOR_LAYERS: i32 = 4;
const FLOOR_BASE: i32 = 2;
const CEILING_LAYERS: i32 = 5;
const FLOOR_SEED_X: u32 = 0x9939_f508;
const FLOOR_SEED_Z: u32 = 0xf156_5bd5;

pub fn chunk_random(chunk_x: i32, chunk_z: i32) -> MersenneTwister {
    MersenneTwister::new((chunk_x as u32).wrapping_mul(FLOOR_SEED_X).wrapping_add((chunk_z as u32).wrapping_mul(FLOOR_SEED_Z)))
}

pub fn place_bedrock(random: &mut MersenneTwister, mut set: impl FnMut(i32, i32, i32)) {
    for x in 0..16 {
        for z in 0..16 {
            for y in 0..random.next_int_bounded(FLOOR_LAYERS) + FLOOR_BASE {
                set(x, y, z);
            }
        }
    }
    for x in 0..16 {
        for z in 0..16 {
            for y in TERRAIN_HEIGHT - 1 - random.next_int_bounded(CEILING_LAYERS)..TERRAIN_HEIGHT {
                set(x, y, z);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::biome::{BASALT_DELTAS, SOULSAND_VALLEY};
    use super::*;
    use chorus_block::block_registry::BlockRegistry;
    use std::sync::LazyLock;

    static REGISTRY: LazyLock<BlockRegistry> = LazyLock::new(|| {
        let mut registry = BlockRegistry::new();
        registry.register_all(chorus_block::r#impl::DEFINITIONS.iter().copied());
        registry
    });

    fn parsed() -> (NetherSurface, BlockTable<'static>) {
        let mut table = BlockTable::new(&REGISTRY);
        let surface = NetherSurface::new(1234567, &mut table);
        (surface, table)
    }

    fn capped_of(surface: &NetherSurface, biome: i32) -> &Capped {
        match surface.builder(biome) {
            Some(Builder::Capped(capped)) => capped,
            _ => panic!("biome {biome} has no capped builder"),
        }
    }

    #[test]
    fn the_capped_builders_hold_what_the_documents_say() {
        let (surface, mut table) = parsed();
        let mut id = |name: &'static str| table.name(name);
        let (soul_sand, soul_soil, gravel, lava, netherrack) = (
            id("minecraft:soul_sand"),
            id("minecraft:soul_soil"),
            id("minecraft:gravel"),
            id("minecraft:lava"),
            id("minecraft:netherrack"),
        );
        let (basalt, blackstone) = (id("minecraft:basalt"), id("minecraft:blackstone"));

        let soul = capped_of(&surface, SOULSAND_VALLEY);
        assert_eq!(soul.floor, [soul_sand, soul_soil]);
        assert_eq!(soul.ceiling, [soul_sand, soul_soil]);
        assert_eq!((soul.beach, soul.sea, soul.foundation), (Some(gravel), lava, netherrack));
        assert_eq!((soul.noises.floor.len(), soul.noises.ceiling.len()), (2, 2));

        let delta = capped_of(&surface, BASALT_DELTAS);
        assert_eq!(delta.floor, [basalt, blackstone]);
        assert_eq!(delta.ceiling, [basalt]);
        assert_eq!((delta.beach, delta.sea, delta.foundation), (None, lava, netherrack));
        assert_eq!((delta.noises.floor.len(), delta.noises.ceiling.len()), (2, 1));
    }
}
