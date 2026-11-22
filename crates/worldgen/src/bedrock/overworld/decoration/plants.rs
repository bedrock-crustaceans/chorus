use super::ore::Draw;
use super::world::{is_air, read};
use crate::bedrock::overworld::surface::info_noise;
use crate::shared::blocks::{BlockId, BlockTable, flag, state, text};
use crate::shared::chunk::region::Region;
use crate::shared::survival::can_survive;
use chorus_block::block_id::*;
use chorus_block::state::common::{BAMBOO_LEAF_SIZE, UPPER_BLOCK_BIT};
use glam::IVec3;

const CACTUS_ATTEMPTS: usize = 10;
const CACTUS_HEIGHT_BOUND: i32 = 3;
const CACTUS_FLOWER_BOUND: i32 = 3;
const CACTUS_FLOWER_ENABLED: bool = true;
const SPREAD_MASK: u32 = 7;
const SPREAD_Y_MASK: u32 = 3;
const BAMBOO_HASH_X: u32 = 0x2fc20f;
const BAMBOO_HASH_Z: u32 = 0x6ebfff5;
const BAMBOO_MIX_MULTIPLIER: u32 = 0x285b825;
const BAMBOO_MIX_ADD: u32 = 0xb;
const BAMBOO_BOUND_SHIFT: u32 = 24;
const BAMBOO_DIVIDE_MAGIC: u32 = 205;
const BAMBOO_DIVIDE_SHIFT: u32 = 10;
const BAMBOO_DIVISOR: u32 = 5;
const BAMBOO_BOUND_FLOOR: u32 = 8;
const BAMBOO_MIN_ROLL: i32 = -4;
const BAMBOO_HEIGHT_BASE: i32 = 4;
const BAMBOO_LEAF_MIN_GAIN: i32 = 3;
const BAMBOO_LEAF_COUNT: i32 = 3;
const BAMBOO_LEAF_STATE: &str = "large_leaves";
const BAMBOO_SMALL_LEAF_STATE: &str = "small_leaves";
const BAMBOO_LARGE_LEAVES: i32 = 2;
const BAMBOO_SUBSTRATES: [&str; 3] = [GRASS_BLOCK, PODZOL, DIRT];
const FLOWER_ATTEMPTS: usize = 1;
const FOREST_BOUND: i32 = 3;
const PLAINS_TULIP_NOISE_LIMIT: f64 = -0.8;
const PLAINS_NOISE_SCALE: f32 = 200.0;
const PLAINS_BOUND: i32 = 4;
const FLOWER_FOREST_NOISE_SCALE: f32 = 48.0;
const NOISE_UNIT_MAX: f32 = 0.9999;
const MEADOW_SLOW_SCALE: f32 = 0.0005;
const MEADOW_FAST_SCALE: f32 = 0.1;
const MEADOW_PALETTE_SALT: i32 = 0x3936d;
const MEADOW_UNIT_MAX: f32 = 0.999_999_9;
const MEADOW_MAX_VARIETY: i32 = 4;
const MEADOW_BLOCKS: [&str; 8] = [ALLIUM, POPPY, CORNFLOWER, OXEYE_DAISY, AZURE_BLUET, DANDELION, SHORT_GRASS, TALL_GRASS];
const TULIPS: [&str; 4] = [RED_TULIP, ORANGE_TULIP, WHITE_TULIP, PINK_TULIP];
const PLAINS_PICKS: [&str; 4] = [POPPY, AZURE_BLUET, OXEYE_DAISY, CORNFLOWER];
const FLOWER_FOREST_PALETTE: [&str; 11] = [
    POPPY,
    BLUE_ORCHID,
    ALLIUM,
    AZURE_BLUET,
    RED_TULIP,
    ORANGE_TULIP,
    WHITE_TULIP,
    PINK_TULIP,
    OXEYE_DAISY,
    CORNFLOWER,
    LILY_OF_THE_VALLEY,
];
pub fn spread<D: Draw>(random: &mut D) -> IVec3 {
    let mut words = [0u32; 6];
    words.iter_mut().for_each(|word| *word = random.word());
    IVec3::new(
        (words[4] & SPREAD_MASK) as i32 - (words[5] & SPREAD_MASK) as i32,
        (words[2] & SPREAD_Y_MASK) as i32 - (words[3] & SPREAD_Y_MASK) as i32,
        (words[0] & SPREAD_MASK) as i32 - (words[1] & SPREAD_MASK) as i32,
    )
}

#[derive(Debug)]
pub struct Cactus {
    shaft: BlockId,
    flower: Option<BlockId>,
}

impl Cactus {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            shaft: table.name(CACTUS),
            flower: CACTUS_FLOWER_ENABLED.then(|| table.name(CACTUS_FLOWER)),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        for _ in 0..CACTUS_ATTEMPTS {
            let base = origin + spread(random);
            if !is_air(region, base) {
                continue;
            }
            let roll = random.below(CACTUS_HEIGHT_BOUND);
            let height = random.below(roll + 1) + 1;
            let mut placed = 0;
            for offset in 0..height {
                let pos = base + IVec3::Y * offset;
                if !can_survive(region, self.shaft, pos) {
                    break;
                }
                region.set(pos, self.shaft);
                placed += 1;
            }
            if let Some(flower) = self.flower
                && placed > 0
                && random.below(CACTUS_FLOWER_BOUND) == 0
            {
                let top = base + IVec3::Y * placed;
                if is_air(region, top) {
                    region.set(top, flower);
                }
            }
        }
        true
    }
}

#[derive(Debug)]
pub struct Bamboo {
    stalk: BlockId,
    leaf: BlockId,
    small_leaf: BlockId,
    substrates: Vec<u16>,
}

pub fn bamboo_bound(x: i32, z: i32) -> i32 {
    let hash = (z as u32).wrapping_mul(BAMBOO_HASH_Z) ^ (x as u32).wrapping_mul(BAMBOO_HASH_X);
    let mixed = hash.wrapping_mul(BAMBOO_MIX_MULTIPLIER).wrapping_add(BAMBOO_MIX_ADD).wrapping_mul(hash);
    let top = mixed >> BAMBOO_BOUND_SHIFT;
    let remainder = top.wrapping_sub(((top * BAMBOO_DIVIDE_MAGIC) >> BAMBOO_DIVIDE_SHIFT) * BAMBOO_DIVISOR) & 0xff;
    (remainder | BAMBOO_BOUND_FLOOR) as i32
}

impl Bamboo {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            stalk: table.name(BAMBOO),
            leaf: table.get(BAMBOO, &[state(&BAMBOO_LEAF_SIZE, text(BAMBOO_LEAF_STATE))]),
            small_leaf: table.get(BAMBOO, &[state(&BAMBOO_LEAF_SIZE, text(BAMBOO_SMALL_LEAF_STATE))]),
            substrates: BAMBOO_SUBSTRATES.iter().map(|name| table.kind(*name)).collect(),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        if !is_air(region, origin) {
            return false;
        }
        if !can_survive(region, self.stalk, origin) {
            return false;
        }
        let below = read(region, origin - IVec3::Y);
        if !self.substrates.contains(&region.blocks.entry(below).kind) {
            return false;
        }
        let roll = random.below(bamboo_bound(origin.x, origin.z));
        if roll < BAMBOO_MIN_ROLL {
            return false;
        }
        let count = (roll + BAMBOO_HEIGHT_BASE).max(0) + 1;
        let mut top = origin.y;
        for _ in 0..count {
            let pos = IVec3::new(origin.x, top, origin.z);
            if !is_air(region, pos) {
                break;
            }
            region.set(pos, self.stalk);
            top += 1;
        }
        if top - origin.y >= BAMBOO_LEAF_MIN_GAIN {
            for step in 0..BAMBOO_LEAF_COUNT {
                let leaf = if step < BAMBOO_LARGE_LEAVES { self.leaf } else { self.small_leaf };
                region.set(IVec3::new(origin.x, top - step, origin.z), leaf);
            }
        }
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowerMode {
    FlowerForest,
    Forest,
    Default,
    Plains,
    Swamp,
}

#[derive(Debug)]
pub struct Flower {
    mode: FlowerMode,
    dandelion: BlockId,
    poppy: BlockId,
    lily: BlockId,
    orchid: BlockId,
    palette: Vec<BlockId>,
    tulips: Vec<BlockId>,
    plains: Vec<BlockId>,
}

impl Flower {
    pub fn new(table: &mut BlockTable, mode: FlowerMode) -> Self {
        Self {
            mode,
            dandelion: table.name(DANDELION),
            poppy: table.name(POPPY),
            lily: table.name(LILY_OF_THE_VALLEY),
            orchid: table.name(BLUE_ORCHID),
            palette: FLOWER_FOREST_PALETTE.iter().map(|name| table.name(*name)).collect(),
            tulips: TULIPS.iter().map(|name| table.name(*name)).collect(),
            plains: PLAINS_PICKS.iter().map(|name| table.name(*name)).collect(),
        }
    }

    pub fn pick<D: Draw>(&self, random: &mut D, origin: IVec3) -> BlockId {
        match self.mode {
            FlowerMode::FlowerForest => {
                let noise = info_noise(origin.x as f32 / FLOWER_FOREST_NOISE_SCALE, origin.z as f32 / FLOWER_FOREST_NOISE_SCALE);
                let unit = ((noise + 1.0) * 0.5).clamp(0.0, NOISE_UNIT_MAX);
                let block = self.palette[(unit * self.palette.len() as f32) as usize];
                if block == self.orchid { self.poppy } else { block }
            }
            FlowerMode::Forest => {
                if random.boolean() {
                    self.lily
                } else {
                    self.dandelion
                }
            }
            FlowerMode::Default => {
                if random.below(FOREST_BOUND) == 0 {
                    self.dandelion
                } else {
                    self.poppy
                }
            }
            FlowerMode::Plains => {
                let noise = info_noise(origin.x as f32 / PLAINS_NOISE_SCALE, origin.z as f32 / PLAINS_NOISE_SCALE);
                if (noise as f64) < PLAINS_TULIP_NOISE_LIMIT {
                    self.tulips[random.below(PLAINS_BOUND) as usize]
                } else if random.below(PLAINS_BOUND) > 0 {
                    self.plains[random.below(PLAINS_BOUND) as usize]
                } else {
                    self.dandelion
                }
            }
            FlowerMode::Swamp => self.orchid,
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        if !is_air(region, origin) {
            return false;
        }
        let block = self.pick(random, origin);
        let mut at = origin;
        for _ in 0..FLOWER_ATTEMPTS {
            if is_air(region, at) && can_survive(region, block, at) {
                region.set(at, block);
            }
            at = origin + spread(random);
        }
        true
    }
}

#[derive(Debug)]
pub struct Meadow {
    blocks: Vec<BlockId>,
    tall_top: BlockId,
    tall_grass: BlockId,
}

fn unit_noise(x: f32, z: f32) -> f32 {
    (info_noise(x, z) + 1.0) * 0.5
}

impl Meadow {
    pub fn new(table: &mut BlockTable) -> Self {
        let tall_grass = table.get(TALL_GRASS, &[state(&UPPER_BLOCK_BIT, flag(false))]);
        Self {
            blocks: MEADOW_BLOCKS.iter().map(|&name| if name == TALL_GRASS { tall_grass } else { table.name(name) }).collect(),
            tall_top: table.get(TALL_GRASS, &[state(&UPPER_BLOCK_BIT, flag(true))]),
            tall_grass,
        }
    }

    fn variety(x: i32, z: i32) -> i32 {
        let slow = unit_noise(x as f32 * MEADOW_SLOW_SCALE, z as f32 * MEADOW_SLOW_SCALE);
        if slow < 0.0 {
            1
        } else if slow > 1.0 {
            MEADOW_MAX_VARIETY
        } else {
            (slow * (MEADOW_MAX_VARIETY - 1) as f32 + 1.0) as i32
        }
    }

    pub fn place(&self, region: &mut Region, pos: IVec3) -> bool {
        let variety = Self::variety(pos.x, pos.z);
        let palette: Vec<BlockId> = (0..variety)
            .map(|index| {
                let shifted = index.wrapping_mul(MEADOW_PALETTE_SALT).wrapping_add(pos.x);
                let unit = unit_noise(shifted as f32 * MEADOW_SLOW_SCALE, pos.z as f32 * MEADOW_SLOW_SCALE).clamp(0.0, MEADOW_UNIT_MAX);
                self.blocks[(unit * self.blocks.len() as f32) as usize]
            })
            .collect();
        let chosen = if palette.len() == 1 {
            0
        } else {
            (unit_noise(pos.x as f32 * MEADOW_FAST_SCALE, pos.z as f32 * MEADOW_FAST_SCALE).clamp(0.0, MEADOW_UNIT_MAX) * palette.len() as f32) as usize
        };
        let block = palette[chosen];
        if !can_survive(region, block, pos) || !is_air(region, pos) {
            return false;
        }
        if block == self.tall_grass {
            region.set(pos + IVec3::Y, self.tall_top);
        }
        region.set(pos, block)
    }
}
