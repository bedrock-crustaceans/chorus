use super::ore::Draw;
use super::world::{is_air, is_solid, read};
use crate::shared::blocks::{BlockId, BlockTable, state, text};
use crate::shared::chunk::proto::BlockEntity;
use crate::shared::chunk::region::Region;
use chorus_block::block_id::*;
use chorus_block::state::common::MINECRAFT_CARDINAL_DIRECTION;
use glam::IVec3;

pub(super) const BOX_XZ: i32 = 16;
pub(super) const BOX_Y: i32 = 8;
const BOX_CELLS: usize = (BOX_XZ * BOX_XZ * BOX_Y) as usize;
const BOX_MARGIN: i32 = 8;
const MIN_HEIGHT_MARGIN: i32 = 4;
const DESCENT_FLOOR: i32 = 1;
const BLOB_BASE: i32 = 4;
const BLOB_VARIATION: i32 = 4;
const BLOB_INNER_MIN: i32 = 1;
const BLOB_INNER_MAX: i32 = 15;
const BLOB_LEVEL_MAX: i32 = 6;
const BLOB_RADIUS_XZ: f32 = 6.0;
const BLOB_RADIUS_XZ_BASE: f32 = 3.0;
const BLOB_RADIUS_Y: f32 = 4.0;
const BLOB_RADIUS_Y_BASE: f32 = 2.0;
const BLOB_CENTRE_XZ_SPAN: f32 = 16.0;
const BLOB_CENTRE_XZ_PAD: f32 = -2.0;
const BLOB_CENTRE_Y_SPAN: f32 = 8.0;
const BLOB_CENTRE_Y_PAD: f32 = -4.0;
const BLOB_CENTRE_XZ_BASE: f32 = 1.0;
const BLOB_CENTRE_Y_BASE: f32 = 2.0;
const BLOB_HALF: f32 = 0.5;
pub(super) const FLUID_LEVELS: i32 = 4;
const BARRIER_DRAW_BOUND: i32 = 2;

pub(super) struct LakeShape {
    pub base: IVec3,
    cells: [bool; BOX_CELLS],
}

impl LakeShape {
    pub fn build<D: Draw>(region: &Region, random: &mut D, origin: IVec3) -> Option<Self> {
        if region.min_y() + MIN_HEIGHT_MARGIN >= origin.y {
            return None;
        }
        let (corner_x, corner_z) = (origin.x - BOX_MARGIN, origin.z - BOX_MARGIN);
        let mut y = origin.y;
        while y > region.min_y() && cell(region, IVec3::new(corner_x, y, corner_z)) == Cell::Air && y >= DESCENT_FLOOR {
            y -= 1;
        }
        let base = IVec3::new(corner_x, y - MIN_HEIGHT_MARGIN, corner_z);
        let mut cells = [false; BOX_CELLS];
        for _ in 0..random.below(BLOB_VARIATION) + BLOB_BASE {
            add_blob(random, &mut cells);
        }
        Some(Self { base, cells })
    }

    pub fn filled(&self, x: i32, z: i32, y: i32) -> bool {
        filled(&self.cells, x, z, y)
    }

    pub fn borders(&self, x: i32, z: i32, y: i32) -> bool {
        borders(&self.cells, x, z, y)
    }
}

pub const LAVA_CHANCE: i32 = 8;
const LAVA_ORIGIN_SPAN: i32 = 9;
const LAVA_ORIGIN_SHIFT: i32 = 3;
const LAVA_HEIGHT_BOUND: i32 = 120;
const LAVA_HEIGHT_BASE: i32 = 8;
const LAVA_UNDER_SEA_MARGIN: i32 = 3;
const LAVA_ABOVE_SEA_CHANCE: i32 = 10;
const ROOM_ORIGIN_SPAN: i32 = 16;
const ROOM_ORIGIN_SHIFT: i32 = 8;
const ROOM_GENERATION_HEIGHT: i32 = 320;
const ROOM_DEEP_COUNT: i32 = 4;
const ROOM_DEEP_FLOOR_MARGIN: i32 = 6;
const ROOM_COUNT: i32 = 10;
const ROOM_RADIUS_BOUND: i32 = 2;
const ROOM_RADIUS_BASE: i32 = 2;
const ROOM_HOLES_MIN: i32 = 1;
const ROOM_HOLES_MAX: i32 = 5;
const ROOM_MOSS_BOUND: i32 = 4;
const ROOM_CHEST_ROUNDS: i32 = 2;
const ROOM_CHEST_TRIES: i32 = 3;
const ROOM_LOOT_TABLE: &str = "simple_dungeon";
const ROOM_LOOT_SEED: i64 = 0;
const CHEST_FACINGS: [&str; 4] = chorus_block::state::common::CARDINAL_DIRECTIONS;
const SIDES: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const ROOM_MOBS: [(&str, f32, f32); 4] = [
    ("minecraft:skeleton", 0.6, 1.99),
    ("minecraft:zombie", 0.6, 1.95),
    ("minecraft:zombie", 0.6, 1.95),
    ("minecraft:spider", 1.4, 0.9),
];

pub fn draw_lava_origin<D: Draw>(random: &mut D, chunk_x: i32, chunk_z: i32, sea_level: i32) -> Option<IVec3> {
    if random.below(LAVA_CHANCE) != 0 {
        return None;
    }
    let x = random.below(LAVA_ORIGIN_SPAN) + (chunk_x << 4) + LAVA_ORIGIN_SHIFT;
    let bound = random.below(LAVA_HEIGHT_BOUND) + LAVA_HEIGHT_BASE;
    let y = random.below(bound);
    let z = random.below(LAVA_ORIGIN_SPAN) + (chunk_z << 4) + LAVA_ORIGIN_SHIFT;
    let accepted = y >= sea_level - LAVA_UNDER_SEA_MARGIN && (y <= sea_level || random.below(LAVA_ABOVE_SEA_CHANCE) == 0);
    accepted.then_some(IVec3::new(x, y, z))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Cell {
    Air,
    Water,
    Lava,
    Solid,
}

pub(super) fn cell(region: &Region, pos: IVec3) -> Cell {
    let id = read(region, pos);
    let entry = region.blocks.entry(id);
    if region.blocks.is_air(id) {
        Cell::Air
    } else if entry.name.as_ref() == LAVA {
        Cell::Lava
    } else if entry.liquid {
        Cell::Water
    } else {
        Cell::Solid
    }
}

fn index(x: i32, z: i32, y: i32) -> usize {
    ((x * BOX_XZ + z) * BOX_Y + y) as usize
}

fn filled(mask: &[bool; BOX_CELLS], x: i32, z: i32, y: i32) -> bool {
    (0..BOX_XZ).contains(&x) && (0..BOX_XZ).contains(&z) && (0..BOX_Y).contains(&y) && mask[index(x, z, y)]
}

fn borders(mask: &[bool; BOX_CELLS], x: i32, z: i32, y: i32) -> bool {
    !filled(mask, x, z, y)
        && [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)]
            .iter()
            .any(|&(dx, dz, dy)| filled(mask, x + dx, z + dz, y + dy))
}

fn add_blob<D: Draw>(random: &mut D, mask: &mut [bool; BOX_CELLS]) {
    let size_x = random.float() * BLOB_RADIUS_XZ + BLOB_RADIUS_XZ_BASE;
    let size_y = random.float() * BLOB_RADIUS_Y + BLOB_RADIUS_Y_BASE;
    let size_z = random.float() * BLOB_RADIUS_XZ + BLOB_RADIUS_XZ_BASE;
    let centre_x = ((BLOB_CENTRE_XZ_SPAN - size_x) + BLOB_CENTRE_XZ_PAD) * random.float() + BLOB_CENTRE_XZ_BASE + size_x * BLOB_HALF;
    let centre_y = ((BLOB_CENTRE_Y_SPAN - size_y) + BLOB_CENTRE_Y_PAD) * random.float() + BLOB_CENTRE_Y_BASE + size_y * BLOB_HALF;
    let centre_z = ((BLOB_CENTRE_XZ_SPAN - size_z) + BLOB_CENTRE_XZ_PAD) * random.float() + BLOB_CENTRE_XZ_BASE + size_z * BLOB_HALF;
    let (half_x, half_y, half_z) = (size_x * BLOB_HALF, size_y * BLOB_HALF, size_z * BLOB_HALF);
    for x in BLOB_INNER_MIN..BLOB_INNER_MAX {
        let dx = (x as f32 - centre_x) / half_x;
        let dx_squared = dx * dx;
        for z in BLOB_INNER_MIN..BLOB_INNER_MAX {
            let dz = (z as f32 - centre_z) / half_z;
            let dz_squared = dz * dz;
            for y in BLOB_INNER_MIN..=BLOB_LEVEL_MAX {
                let dy = (y as f32 - centre_y) / half_y;
                let dy_squared = dy * dy;
                if (dy_squared + dx_squared) + dz_squared < 1.0 {
                    mask[index(x, z, y)] = true;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct LavaLake {
    lava: BlockId,
    air: BlockId,
    barrier: BlockId,
}

impl LavaLake {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            lava: table.name(LAVA),
            air: table.name(AIR),
            barrier: table.name(STONE),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        region.begin_feature();
        region.set_any_rule();
        let Some(shape) = LakeShape::build(region, random, origin) else { return false };
        let base = shape.base;

        for x in 0..BOX_XZ {
            for z in 0..BOX_XZ {
                for y in 0..BOX_Y {
                    if !shape.borders(x, z, y) {
                        continue;
                    }
                    let found = cell(region, base + IVec3::new(x, y, z));
                    let liquid = matches!(found, Cell::Water | Cell::Lava);
                    if (y >= FLUID_LEVELS && liquid) || (y < FLUID_LEVELS && found != Cell::Solid && found != Cell::Lava) {
                        return false;
                    }
                }
            }
        }

        let mut barrier = Vec::new();
        for x in 0..BOX_XZ {
            for z in 0..BOX_XZ {
                for y in 0..BOX_Y {
                    if shape.borders(x, z, y) && (y < FLUID_LEVELS || random.below(BARRIER_DRAW_BOUND) != 0) && cell(region, base + IVec3::new(x, y, z)) == Cell::Solid {
                        barrier.push(base + IVec3::new(x, y, z));
                    }
                }
            }
        }
        for x in 0..BOX_XZ {
            for z in 0..BOX_XZ {
                for y in 0..BOX_Y {
                    if shape.filled(x, z, y) {
                        region.set(base + IVec3::new(x, y, z), if y >= FLUID_LEVELS { self.air } else { self.lava });
                    }
                }
            }
        }
        for pos in barrier {
            region.set(pos, self.barrier);
        }
        true
    }
}

#[derive(Debug)]
pub struct MonsterRoom {
    air: BlockId,
    cobblestone: BlockId,
    mossy: BlockId,
    chests: [BlockId; 4],
    spawner: BlockId,
}

pub(super) fn chest_facing(open: impl Fn(i32, i32) -> bool, dx: i32, dz: i32) -> usize {
    let behind = match dz.signum() {
        -1 if !open(0, -1) => return 0,
        -1 => true,
        1 if !open(0, 1) => return 2,
        _ => false,
    };
    let toward = || {
        if !behind && dz <= 0 { if dx > 0 { 1 } else { 2 } } else { usize::from(dz > 0) * 2 }
    };
    match dx.signum() {
        -1 => {
            let blocked = !open(-1, 0);
            if behind || dz > 0 || blocked { if blocked { 3 } else { usize::from(dz > 0) * 2 } } else { 3 }
        }
        1 if !open(1, 0) => 1,
        _ => toward(),
    }
}

impl MonsterRoom {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            air: table.name(AIR),
            cobblestone: table.name(COBBLESTONE),
            mossy: table.name(MOSSY_COBBLESTONE),
            chests: CHEST_FACINGS.map(|facing| table.get(CHEST, &[state(&MINECRAFT_CARDINAL_DIRECTION, text(facing))])),
            spawner: table.name(MOB_SPAWNER),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        region.begin_feature();
        let xr = random.below(ROOM_RADIUS_BOUND) + ROOM_RADIUS_BASE;
        let zr = random.below(ROOM_RADIUS_BOUND) + ROOM_RADIUS_BASE;
        let (min_x, max_x, min_z, max_z) = (-xr - 1, xr + 1, -zr - 1, zr + 1);
        let mut holes = 0;
        for dx in min_x..=max_x {
            for dy in -1..=4 {
                for dz in min_z..=max_z {
                    let pos = origin + IVec3::new(dx, dy, dz);
                    if (dy == -1 || dy == 4) && !is_solid(region, pos) {
                        return false;
                    }
                    if (dx == min_x || dx == max_x || dz == min_z || dz == max_z) && dy == 0 && is_air(region, pos) && is_air(region, pos + IVec3::Y) {
                        holes += 1;
                    }
                }
            }
        }
        if !(ROOM_HOLES_MIN..=ROOM_HOLES_MAX).contains(&holes) {
            return false;
        }
        for dx in min_x..=max_x {
            for dy in (-1..=3).rev() {
                for dz in min_z..=max_z {
                    let pos = origin + IVec3::new(dx, dy, dz);
                    let shell = dx == min_x || dx == max_x || dz == min_z || dz == max_z || dy == -1;
                    if !shell || (pos.y >= region.min_y() && !is_solid(region, pos - IVec3::Y)) {
                        region.set(pos, self.air);
                    } else if is_solid(region, pos) {
                        let mossy = dy == -1 && random.below(ROOM_MOSS_BOUND) != 0;
                        region.set(pos, if mossy { self.mossy } else { self.cobblestone });
                    }
                }
            }
        }
        for _ in 0..ROOM_CHEST_ROUNDS {
            for _ in 0..ROOM_CHEST_TRIES {
                let dz = random.below(zr * 2 + 1) - zr;
                let dx = random.below(xr * 2 + 1) - xr;
                let pos = IVec3::new(origin.x + dx, origin.y, origin.z + dz);
                if !is_air(region, pos) {
                    continue;
                }
                let sides = SIDES.iter().filter(|&&(ox, oz)| is_solid(region, pos + IVec3::new(ox, 0, oz))).count();
                if sides == 1 {
                    let facing = chest_facing(|ox, oz| is_air(region, pos + IVec3::new(ox, 0, oz)), dx, dz);
                    region.set(pos, self.chests[facing]);
                    region.set_block_entity(
                        pos,
                        BlockEntity::Chest {
                            loot_table: ROOM_LOOT_TABLE,
                            seed: ROOM_LOOT_SEED,
                        },
                    );
                    random.word();
                    break;
                }
            }
        }
        region.set(origin, self.spawner);
        let (entity, width, height) = ROOM_MOBS[random.below(ROOM_MOBS.len() as i32) as usize];
        region.set_block_entity(origin, BlockEntity::Spawner { entity, width, height });
        true
    }
}

pub fn room_origins<D: Draw>(random: &mut D, chunk_x: i32, chunk_z: i32, min_y: i32, mut visit: impl FnMut(&mut D, IVec3)) {
    let origin = |random: &mut D, height: &dyn Fn(&mut D) -> i32| {
        let x = random.below(ROOM_ORIGIN_SPAN) + (chunk_x << 4) + ROOM_ORIGIN_SHIFT;
        let y = height(random);
        let z = random.below(ROOM_ORIGIN_SPAN) + (chunk_z << 4) + ROOM_ORIGIN_SHIFT;
        IVec3::new(x, y, z)
    };
    let floor = min_y + ROOM_DEEP_FLOOR_MARGIN;
    for _ in 0..ROOM_DEEP_COUNT {
        let found = origin(random, &|random| random.below(floor.abs()) + floor);
        visit(random, found);
    }
    for _ in 0..ROOM_COUNT {
        let found = origin(random, &|random| random.below(ROOM_GENERATION_HEIGHT));
        visit(random, found);
    }
}
