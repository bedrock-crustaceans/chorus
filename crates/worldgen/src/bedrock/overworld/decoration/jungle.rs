use super::aquatic::{is_solid_blocking, is_water};
use super::ore::Draw;
use super::world::{is_air, read};
use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{BlockId, BlockTable, int, state};
use crate::shared::chunk::region::{Heightmap, Region};
use chorus_block::block_id::{DIRT, GRASS_BLOCK, PODZOL, SEA_PICKLE, VINE};
use chorus_block::state::common::{CLUSTER_COUNT, VINE_DIRECTION_BITS};
use glam::IVec3;

const VINE_START_ABOVE_SEA: i32 = 1;
const VINE_REACH: i32 = 3;
const VINE_SHIFT_MASK: u32 = 3;
const VINE_SUPPORTS: [(i32, i32, i32); 4] = [(0, 1, 1), (0, -1, 4), (1, 0, 8), (-1, 0, 2)];
const PODZOL_RADIUS_BOUND: i32 = 4;
const PICKLE_TRIES: usize = 4;
const PICKLE_SPAN: i32 = 6;
const PICKLE_CLUSTERS: i32 = 4;

#[derive(Debug)]
pub struct Vines {
    faces: [BlockId; 4],
}

fn chunk_top(region: &mut Region, chunk_x: i32, chunk_z: i32) -> Option<i32> {
    let (corner_x, corner_z) = (chunk_x * CHUNK_WIDTH, chunk_z * CHUNK_WIDTH);
    if !region.contains(IVec3::new(corner_x, region.min_y(), corner_z)) {
        return None;
    }
    let mut top = region.min_y();
    for dx in 0..CHUNK_WIDTH {
        for dz in 0..CHUNK_WIDTH {
            let set = region.height(Heightmap::WorldSurface, corner_x + dx, corner_z + dz) - 1;
            let built = region.height(Heightmap::WorldSurfaceWg, corner_x + dx, corner_z + dz);
            top = top.max(set).max(built);
        }
    }
    Some(top)
}

impl Vines {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            faces: VINE_SUPPORTS.map(|(_, _, bits)| table.get(VINE, &[state(&VINE_DIRECTION_BITS, int(bits))])),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        let mut top = 0;
        let mut seen: Vec<(i32, i32)> = Vec::new();
        for cx in origin.x - VINE_REACH..=origin.x + VINE_REACH {
            for cz in origin.z - VINE_REACH..=origin.z + VINE_REACH {
                let chunk = (cx.div_euclid(CHUNK_WIDTH), cz.div_euclid(CHUNK_WIDTH));
                if !seen.contains(&chunk) {
                    seen.push(chunk);
                    if let Some(height) = chunk_top(region, chunk.0, chunk.1) {
                        top = top.max(height);
                    }
                }
            }
        }
        if top == 0 {
            top = region.max_y() + 1;
        }
        let (mut column_x, mut column_z) = (origin.x, origin.z);
        for y in region.sea_level() + VINE_START_ABOVE_SEA..top {
            let pos = IVec3::new(column_x, y, column_z);
            if is_air(region, pos) {
                if let Some(index) = VINE_SUPPORTS.iter().position(|&(dx, dz, _)| holds_vine(region, pos + IVec3::new(dx, 0, dz))) {
                    region.set(pos, self.faces[index]);
                }
            } else {
                column_x = origin.x + shift(random);
                column_z = origin.z + shift(random);
            }
        }
        true
    }
}

fn shift<D: Draw>(random: &mut D) -> i32 {
    let high = (random.word() & VINE_SHIFT_MASK) as i32;
    high - (random.word() & VINE_SHIFT_MASK) as i32
}

fn holds_vine(region: &Region, pos: IVec3) -> bool {
    let block = read(region, pos);
    region.blocks.entry(block).vine_support && !region.is_partial_template(block)
}

#[derive(Debug)]
pub struct PodzolArea {
    podzol: BlockId,
    replaced: [u16; 2],
}

impl PodzolArea {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            podzol: table.name(PODZOL),
            replaced: [table.kind(DIRT), table.kind(GRASS_BLOCK)],
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        let radius = random.below(PODZOL_RADIUS_BOUND) + 1;
        let mut placed = false;
        for x in origin.x - radius..=origin.x + radius {
            for z in origin.z - radius..=origin.z + radius {
                let (dx, dz) = (x - origin.x, z - origin.z);
                let ground = IVec3::new(x, origin.y - 1, z);
                if dx * dx + dz * dz <= radius * radius && self.replaced.iter().any(|&kind| region.blocks.is_kind(read(region, ground), kind)) {
                    region.set(ground, self.podzol);
                    placed = true;
                }
            }
        }
        placed
    }
}

#[derive(Debug)]
pub struct SeaPickle {
    clusters: Vec<BlockId>,
    pickle_kind: u16,
}

impl SeaPickle {
    pub fn new(table: &mut BlockTable) -> Self {
        let dry: Vec<BlockId> = (0..PICKLE_CLUSTERS).map(|count| table.get(SEA_PICKLE, &[state(&CLUSTER_COUNT, int(count))])).collect();
        Self {
            clusters: dry.into_iter().map(|id| table.flooded(id)).collect(),
            pickle_kind: table.kind(SEA_PICKLE),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        let mut floor = origin;
        while !is_solid_blocking(region, floor) && !region.blocks.is_kind(read(region, floor), self.pickle_kind) && floor.y > region.min_y() {
            floor.y -= 1;
        }
        if !is_solid_blocking(region, floor) {
            return false;
        }
        for _ in 0..PICKLE_TRIES {
            let dz = random.below(PICKLE_SPAN);
            let dx = random.below(PICKLE_SPAN);
            let count = random.below(PICKLE_CLUSTERS);
            let mut pos = floor + IVec3::new(dx, 0, dz);
            while pos.y <= region.max_y() && !is_air(region, pos) && !is_water(region, pos) {
                pos.y += 1;
            }
            if is_water(region, pos) && is_solid_blocking(region, pos - IVec3::Y) {
                region.set(pos, self.clusters[count as usize]);
            }
        }
        true
    }
}
