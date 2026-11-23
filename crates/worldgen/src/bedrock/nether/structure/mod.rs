mod bastion;
mod fortress;
mod fortress_blocks;
pub(super) mod portal;

use std::sync::Arc;

use crate::shared::biome::pack_ids::{BASALT_DELTAS, CRIMSON_FOREST, HELL, SOULSAND_VALLEY, WARPED_FOREST};
use crate::shared::blocks::{AIR, BlockId, Blocks};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::layout::PoolCatalogue;
use crate::shared::structure::{BoundingBox, Grid, LargeFeatureSeed, PaletteBlocks, Spread, StructureData, TemplateLibrary};

use super::{NetherBlocks, NetherChunk};
use crate::shared::{CHUNK_MASK, CHUNK_WIDTH};
use chorus_level::dimension_definition::DimensionDefinition;

const SPACING: i32 = 30;
const SEPARATION: i32 = 4;
const SALT: i32 = 30_084_232;
const REACH: i32 = 9;
const START_TRIES: i32 = 6;
const FORTRESS_PICK: i32 = 2;
const FORTRESS_BIOMES: [i32; 5] = [HELL, SOULSAND_VALLEY, CRIMSON_FOREST, WARPED_FOREST, BASALT_DELTAS];
const BASTION_BIOMES: [i32; 4] = [HELL, SOULSAND_VALLEY, CRIMSON_FOREST, WARPED_FOREST];
const POOL_CATALOGUE: &str = "bastion";

pub(super) const NETHER_GRID: Grid = Grid::new(SPACING, SEPARATION, SALT, Spread::Linear);

fn column_box(chunk: (i32, i32), definition: DimensionDefinition) -> BoundingBox {
    BoundingBox::chunk(chunk.0, chunk.1, definition)
}

pub(super) type Cell = (i32, i32, i32);

pub(super) const SOUTH: u8 = 0;
pub(super) const WEST: u8 = 1;
pub(super) const NORTH: u8 = 2;
pub(super) const EAST: u8 = 3;

const STAIRS_BY_ORIENTATION: [[usize; 4]; 4] = [[0, 1, 3, 2], [2, 3, 0, 1], [0, 1, 2, 3], [2, 3, 1, 0]];

pub(super) fn orient_box((x, y, z): Cell, (ox, oy, oz): Cell, (sx, sy, sz): Cell, direction: u8) -> BoundingBox {
    match direction {
        NORTH => BoundingBox::new(x + ox, y + oy, z - sz + 1 + oz, x + sx - 1 + ox, y + sy - 1 + oy, z + oz),
        WEST => BoundingBox::new(x - sz + 1 + oz, y + oy, z + ox, x + oz, y + sy - 1 + oy, z + sx - 1 + ox),
        EAST => BoundingBox::new(x + oz, y + oy, z + ox, x + sz - 1 + oz, y + sy - 1 + oy, z + sx - 1 + ox),
        _ => BoundingBox::new(x + ox, y + oy, z + oz, x + sx - 1 + ox, y + sy - 1 + oy, z + sz - 1 + oz),
    }
}

pub(super) struct Frame {
    pub bb: BoundingBox,
    pub orientation: u8,
    pub depth: i32,
}

impl Frame {
    pub fn world(&self, x: i32, y: i32, z: i32) -> Cell {
        let world_x = match self.orientation {
            WEST => self.bb.x1 - z,
            EAST => self.bb.x0 + z,
            _ => self.bb.x0 + x,
        };
        let world_z = match self.orientation {
            SOUTH => self.bb.z0 + z,
            NORTH => self.bb.z1 - z,
            _ => self.bb.z0 + x,
        };
        (world_x, self.bb.y0 + y, world_z)
    }

    pub fn stairs(&self, family: [BlockId; 4], data: u8) -> BlockId {
        family[STAIRS_BY_ORIENTATION[self.orientation as usize][data as usize]]
    }

    pub fn place(&self, canvas: &mut FortressCanvas, block: BlockId, x: i32, y: i32, z: i32) {
        let (world_x, world_y, world_z) = self.world(x, y, z);
        canvas.set(world_x, world_y, world_z, block);
    }

    pub fn fill(&self, canvas: &mut FortressCanvas, (x0, y0, z0): Cell, (x1, y1, z1): Cell, block: BlockId) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    self.place(canvas, block, x, y, z);
                }
            }
        }
    }

    pub fn fill_down(&self, canvas: &mut FortressCanvas, block: BlockId, x: i32, z: i32) {
        let (world_x, mut world_y, world_z) = self.world(x, -1, z);
        if !canvas.contains(world_x, world_y, world_z) {
            return;
        }
        loop {
            let here = canvas.get(world_x, world_y, world_z);
            if canvas.passes_through(here) && world_y > 1 {
                canvas.set(world_x, world_y, world_z, block);
                world_y -= 1;
            } else {
                break;
            }
        }
    }
}

pub(super) struct FortressCanvas<'a> {
    chunk: &'a mut NetherChunk,
    chunk_x: i32,
    chunk_z: i32,
    blocks: &'a Blocks,
}

impl<'a> FortressCanvas<'a> {
    fn new(chunk: &'a mut NetherChunk, (chunk_x, chunk_z): (i32, i32), blocks: &'a Blocks) -> Self {
        Self { chunk, chunk_x, chunk_z, blocks }
    }

    pub fn contains(&self, x: i32, y: i32, z: i32) -> bool {
        self.chunk.contains_y(y) && x >> 4 == self.chunk_x && z >> 4 == self.chunk_z
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        if self.contains(x, y, z) { self.chunk.get(x & CHUNK_MASK, y, z & CHUNK_MASK) } else { AIR }
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        if self.contains(x, y, z) {
            self.chunk.set(x & CHUNK_MASK, y, z & CHUNK_MASK, block);
        }
    }

    pub fn passes_through(&self, block: BlockId) -> bool {
        super::decorate::thin(self.blocks, block) || self.blocks.entry(block).liquid
    }

    pub fn chest_at(&self, x: i32, y: i32, z: i32) -> bool {
        self.blocks.name_of(self.get(x, y, z)) == chorus_block::block_id::CHEST
    }
}

const CONNECTION_SIDES: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
const STAIR_FACINGS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const NO_CORNER: usize = 0;
const INNER_LEFT: usize = 1;
const INNER_RIGHT: usize = 2;
const OUTER_LEFT: usize = 3;
const OUTER_RIGHT: usize = 4;

fn full_cube(blocks: &Blocks, ids: &NetherBlocks, block: BlockId) -> bool {
    let name = blocks.name_of(block);
    if name == "minecraft:mob_spawner" {
        return true;
    }
    block != ids.nether_wart_plant && blocks.entry(block).collision_cube
}

fn stair_facing(ids: &NetherBlocks, block: BlockId) -> Option<(i32, i32)> {
    ids.nether_brick_stairs.iter().position(|&stair| stair == block).map(|facing| STAIR_FACINGS[facing])
}

fn fence_joins(blocks: &Blocks, ids: &NetherBlocks, block: BlockId, (dx, dz): (i32, i32)) -> bool {
    if blocks.name_of(block) == chorus_block::block_id::NETHER_BRICK_FENCE {
        return true;
    }
    stair_facing(ids, block).is_some_and(|facing| facing == (-dx, -dz)) || full_cube(blocks, ids, block)
}

fn world_block(chunk: &NetherChunk, chunk_x: i32, chunk_z: i32, neighbour: &dyn Fn(i32, i32) -> Arc<NetherChunk>, x: i32, y: i32, z: i32) -> BlockId {
    if !chunk.contains_y(y) {
        return AIR;
    }
    let (target_x, target_z) = (x >> 4, z >> 4);
    if (target_x, target_z) == (chunk_x, chunk_z) {
        chunk.get(x & CHUNK_MASK, y, z & CHUNK_MASK)
    } else {
        neighbour(target_x, target_z).get(x & CHUNK_MASK, y, z & CHUNK_MASK)
    }
}

fn stair_corner(chunk: &NetherChunk, chunk_x: i32, chunk_z: i32, neighbour: &dyn Fn(i32, i32) -> Arc<NetherChunk>, ids: &NetherBlocks, (fx, fz): (i32, i32), (x, y, z): Cell) -> usize {
    let stair_at = |dx: i32, dz: i32| {
        let block = world_block(chunk, chunk_x, chunk_z, neighbour, x + dx, y, z + dz);
        stair_facing(ids, block)
    };
    let takes = |dx: i32, dz: i32| stair_at(dx, dz).is_none_or(|facing| facing != (fx, fz));
    let anticlockwise = (fz, -fx);
    if let Some(facing) = stair_at(fx, fz)
        && (facing.0 != 0) != (fx != 0)
        && takes(-facing.0, -facing.1)
    {
        return if facing == anticlockwise { OUTER_LEFT } else { OUTER_RIGHT };
    }
    if let Some(facing) = stair_at(-fx, -fz)
        && (facing.0 != 0) != (fx != 0)
        && takes(facing.0, facing.1)
    {
        return if facing == anticlockwise { INNER_LEFT } else { INNER_RIGHT };
    }
    NO_CORNER
}

fn section_may_join(chunk: &NetherChunk, section: usize, ids: &NetherBlocks, blocks: &Blocks) -> bool {
    chunk.section_palette(section).is_none_or(|palette| {
        palette.iter().any(|&value| {
            let block = value as BlockId;
            blocks.name_of(block) == chorus_block::block_id::NETHER_BRICK_FENCE || stair_facing(ids, block).is_some()
        })
    })
}

impl NetherStructures {
    pub fn joins_reach_neighbours(&self, chunk: &NetherChunk, ids: &NetherBlocks, blocks: &Blocks) -> bool {
        (0..=((chunk.max_y() - chunk.min_y) >> 4) as usize).any(|section| section_may_join(chunk, section, ids, blocks))
    }

    pub fn join(&self, chunk: &mut NetherChunk, (chunk_x, chunk_z): (i32, i32), neighbour: &dyn Fn(i32, i32) -> Arc<NetherChunk>, ids: &NetherBlocks, blocks: &Blocks) {
        let mut updates = Vec::new();
        {
            let get = |x: i32, y: i32, z: i32| world_block(chunk, chunk_x, chunk_z, neighbour, x, y, z);
            for y in chunk.min_y..=chunk.max_y() {
                let section = ((y - chunk.min_y) >> 4) as usize;
                if !section_may_join(chunk, section, ids, blocks) {
                    continue;
                }
                for z in 0..CHUNK_WIDTH {
                    for x in 0..CHUNK_WIDTH {
                        let (world_x, world_z) = (chunk_x * CHUNK_WIDTH + x, chunk_z * CHUNK_WIDTH + z);
                        let block = chunk.get(x, y, z);
                        if blocks.name_of(block) == chorus_block::block_id::NETHER_BRICK_FENCE {
                            let code = CONNECTION_SIDES
                                .iter()
                                .enumerate()
                                .filter(|(_, side)| {
                                    let (dx, dz) = **side;
                                    fence_joins(blocks, ids, get(world_x + dx, y, world_z + dz), (dx, dz))
                                })
                                .fold(0u16, |code, (side, _)| code | 1 << side);
                            let joined = ids.nether_brick_fence_sides[code as usize];
                            if joined != block {
                                updates.push((x, y, z, joined));
                            }
                        } else if let Some(facing) = stair_facing(ids, block) {
                            let corner = stair_corner(chunk, chunk_x, chunk_z, neighbour, ids, facing, (world_x, y, world_z));
                            let joined = ids.nether_brick_stairs_corners[facing_weirdo(facing)][corner];
                            if joined != block {
                                updates.push((x, y, z, joined));
                            }
                        }
                    }
                }
            }
        }
        for (x, y, z, block) in updates {
            chunk.set(x, y, z, block);
        }
    }
}

fn facing_weirdo(facing: (i32, i32)) -> usize {
    STAIR_FACINGS.iter().position(|&vector| vector == facing).unwrap_or(0)
}

pub struct NetherStructures {
    level_seed: u32,
    data: Option<Arc<StructureData>>,
    definition: DimensionDefinition,
}

#[derive(Default)]
pub struct Starts {
    pub(super) kind: Option<SharedKind>,
    pub(super) fortress: Option<Arc<fortress::Fortress>>,
    pub(super) bastion: Option<Arc<bastion::Bastion>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SharedKind {
    Fortress,
    Bastion,
}

impl NetherStructures {
    pub fn new(seed: i64, data: Option<Arc<StructureData>>, definition: DimensionDefinition) -> Self {
        Self {
            level_seed: seed as u32,
            data,
            definition,
        }
    }

    fn catalogue(&self) -> Option<&PoolCatalogue> {
        self.data.as_ref()?.catalogue(POOL_CATALOGUE)
    }

    fn library(&self) -> Option<&TemplateLibrary> {
        self.data.as_ref()?.library.as_ref()
    }

    pub fn starts_at(&self, chunk: (i32, i32), quart_biome: &dyn Fn(i32, i32) -> i32) -> Starts {
        let (candidate, mut random) = NETHER_GRID.candidate_random(self.level_seed, chunk);
        if candidate != chunk {
            return Starts::default();
        }
        let kind = if random.next_int_bounded(START_TRIES) < FORTRESS_PICK {
            SharedKind::Fortress
        } else {
            SharedKind::Bastion
        };
        let (quart_x, quart_z) = (chunk.0 * 4, chunk.1 * 4);
        let quarts = [(quart_x - 1, quart_z - 1), (quart_x - 1, quart_z), (quart_x, quart_z - 1), (quart_x, quart_z)];
        let biomes: &[i32] = match kind {
            SharedKind::Fortress => &FORTRESS_BIOMES,
            SharedKind::Bastion => &BASTION_BIOMES,
        };
        if !quarts.iter().all(|&(x, z)| biomes.contains(&quart_biome(x, z))) {
            return Starts::default();
        }
        match kind {
            SharedKind::Fortress => Starts {
                kind: Some(kind),
                fortress: Some(Arc::new(fortress::generate(&mut random, chunk))),
                bastion: None,
            },
            SharedKind::Bastion => Starts {
                kind: Some(kind),
                fortress: None,
                bastion: self
                    .catalogue()
                    .zip(self.library())
                    .map(|(catalogue, library)| Arc::new(bastion::generate(&mut random, chunk, catalogue, library))),
            },
        }
    }

    pub fn start_cells_near(&self, chunk: (i32, i32)) -> Vec<(i32, i32)> {
        let region = |value: i32| value.div_euclid(SPACING);
        let mut cells = Vec::new();
        for region_x in region(chunk.0 - REACH)..=region(chunk.0 + REACH) {
            for region_z in region(chunk.1 - REACH)..=region(chunk.1 + REACH) {
                let candidate = NETHER_GRID.candidate(self.level_seed, (region_x * SPACING, region_z * SPACING));
                if (candidate.0 - chunk.0).abs() <= REACH && (candidate.1 - chunk.1).abs() <= REACH {
                    cells.push(candidate);
                }
            }
        }
        cells
    }

    pub fn fortresses_at(&self, chunk: (i32, i32), starts: &[((i32, i32), Arc<Starts>)]) -> Vec<Arc<fortress::Fortress>> {
        let column = column_box(chunk, self.definition);
        let mut found: Vec<((i32, i32), Arc<fortress::Fortress>)> = starts
            .iter()
            .filter(|(start, _)| (start.0 - chunk.0).abs() <= REACH && (start.1 - chunk.1).abs() <= REACH)
            .filter_map(|(start, starts)| {
                let structure = starts.fortress.as_ref().filter(|structure| structure.bb.intersects(&column))?;
                Some((NETHER_GRID.region(*start), structure.clone()))
            })
            .collect();
        found.sort_by_key(|(region, _)| *region);
        found.into_iter().map(|(_, structure)| structure).collect()
    }

    pub fn bastions_at(&self, chunk: (i32, i32), starts: &[((i32, i32), Arc<Starts>)]) -> Vec<Arc<bastion::Bastion>> {
        let column = column_box(chunk, self.definition);
        let mut found: Vec<((i32, i32), Arc<bastion::Bastion>)> = starts
            .iter()
            .filter(|(start, _)| (start.0 - chunk.0).abs() <= bastion::REACH && (start.1 - chunk.1).abs() <= bastion::REACH)
            .filter_map(|(start, starts)| {
                let structure = starts.bastion.as_ref().filter(|structure| structure.reaches(&column))?;
                Some((NETHER_GRID.region(*start), structure.clone()))
            })
            .collect();
        found.sort_by_key(|(region, _)| *region);
        found.into_iter().map(|(_, structure)| structure).collect()
    }

    pub fn place_bastions(
        &self,
        bastions: &[Arc<bastion::Bastion>],
        chunk: (i32, i32),
        base: &mut NetherChunk,
        ids: &NetherBlocks,
        blocks: &Blocks,
        palette: &PaletteBlocks,
        neighbour: &dyn Fn(i32, i32) -> Arc<NetherChunk>,
    ) {
        let (Some(catalogue), Some(library)) = (self.catalogue(), self.library()) else { return };
        let seed = LargeFeatureSeed::new(self.level_seed).chunk(chunk.0, chunk.1);
        let mut random = MersenneTwister::new(seed);
        for structure in bastions {
            bastion::place(structure, base, chunk, ids, blocks, palette, library, catalogue, neighbour, seed, &mut random);
        }
    }

    pub fn place(&self, fortresses: &[Arc<fortress::Fortress>], chunk: (i32, i32), base: &mut NetherChunk, ids: &NetherBlocks, blocks: &Blocks) {
        if fortresses.is_empty() {
            return;
        }
        let mut random = MersenneTwister::new(LargeFeatureSeed::new(self.level_seed).chunk(chunk.0, chunk.1));
        for structure in fortresses {
            fortress_blocks::draw(structure, chunk, base, &mut random, ids, blocks);
        }
    }
}
