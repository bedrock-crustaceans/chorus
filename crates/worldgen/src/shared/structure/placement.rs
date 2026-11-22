use crate::shared::rng::legacy::Legacy;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::rng::random::RandomSource;
use crate::shared::rng::seeds::region_seed;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const JAVA_REGION_X_MULTIPLIER: i64 = 341_873_128_712;
const JAVA_REGION_Z_MULTIPLIER: i64 = 132_897_987_541;

pub const STRONGHOLD_CELL_SIZE: i32 = 200;
const STRONGHOLD_CELL_OFFSET: i32 = 100;
const STRONGHOLD_CELL_X_MULTIPLIER: i32 = -0x6454_14af;
const STRONGHOLD_CELL_Z_MULTIPLIER: i32 = -0x4197_5147;
const STRONGHOLD_SALT: u32 = 0x05d5_34e7;
const STRONGHOLD_SPREAD: u32 = 100;
pub const STRONGHOLD_SPREAD_OFFSET: i32 = 50;
const STRONGHOLD_SPAWN_RADIUS_SQUARED: i32 = 99;
const STRONGHOLD_CHANCE: f32 = 0.25;

const RAW_UNIT: f64 = 2.328_306_436_538_696_3e-10;
const MINESHAFT_CHANCE: f32 = 0.004;
const MINESHAFT_DISTANCE_RANGE: i32 = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spread {
    Linear,
    Triangular,
}

#[derive(Debug, Clone, Copy)]
pub struct Grid {
    pub spacing: i32,
    pub separation: i32,
    pub salt: i32,
    pub spread: Spread,
}

impl Grid {
    pub const fn new(spacing: i32, separation: i32, salt: i32, spread: Spread) -> Self {
        Self { spacing, separation, salt, spread }
    }

    pub fn region(&self, chunk: (i32, i32)) -> (i32, i32) {
        (chunk.0.div_euclid(self.spacing), chunk.1.div_euclid(self.spacing))
    }

    pub fn candidate(&self, level_seed: u32, chunk: (i32, i32)) -> (i32, i32) {
        self.candidate_random(level_seed, chunk).0
    }

    pub fn candidate_random(&self, level_seed: u32, chunk: (i32, i32)) -> ((i32, i32), MersenneTwister) {
        let (region_x, region_z) = self.region(chunk);
        let mut random = MersenneTwister::new(region_seed(level_seed, region_x, region_z, self.salt));
        let range = self.spacing - self.separation;
        let mut draw = || match self.spread {
            Spread::Linear => random.next_int_bounded(range),
            Spread::Triangular => (random.next_int_bounded(range) + random.next_int_bounded(range)) / 2,
        };
        let x = draw();
        let z = draw();
        ((region_x * self.spacing + x, region_z * self.spacing + z), random)
    }

    pub fn java_candidate(&self, seed: i64, chunk: (i32, i32)) -> (i32, i32) {
        let (region_x, region_z) = self.region(chunk);
        let region_seed = (region_x as i64)
            .wrapping_mul(JAVA_REGION_X_MULTIPLIER)
            .wrapping_add((region_z as i64).wrapping_mul(JAVA_REGION_Z_MULTIPLIER))
            .wrapping_add(seed)
            .wrapping_add(self.salt as i64);
        let mut random = Legacy::new(region_seed);
        let range = self.spacing - self.separation;
        let mut draw = || match self.spread {
            Spread::Linear => random.next_int_bounded(range),
            Spread::Triangular => (random.next_int_bounded(range) + random.next_int_bounded(range)) / 2,
        };
        let x = draw();
        let z = draw();
        (region_x * self.spacing + x, region_z * self.spacing + z)
    }
}

const START_CACHE_SLOTS: usize = 1 << 13;
const START_CACHE_SHIFT: u32 = 64 - 13;
const START_CACHE_VALID: u64 = 1 << 63;
const START_CACHE_RESULT: u64 = 1 << 62;
const START_CACHE_KEY: u64 = START_CACHE_RESULT - 1;
const START_CACHE_COORDINATE_LIMIT: i32 = 1 << 30;
const START_CACHE_COORDINATE_MASK: u64 = 0x7FFF_FFFF;
const START_CACHE_MIX: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Debug, Clone)]
pub struct MineshaftSeed {
    level_seed: u32,
    first: i32,
    second: i32,
    starts: Arc<[AtomicU64]>,
}

impl MineshaftSeed {
    pub fn new(level_seed: u32) -> Self {
        let mut random = MersenneTwister::new(level_seed);
        let first = (random.next_u32() >> 1) as i32;
        let second = (random.next_u32() >> 1) as i32;
        Self {
            level_seed,
            first,
            second,
            starts: (0..START_CACHE_SLOTS).map(|_| AtomicU64::new(0)).collect(),
        }
    }

    fn chunk_seed(&self, (x, z): (i32, i32)) -> u32 {
        (self.second.wrapping_mul(z) ^ self.first.wrapping_mul(x)) as u32 ^ self.level_seed
    }

    pub fn is_start(&self, chunk: (i32, i32)) -> bool {
        if chunk.0.abs() >= START_CACHE_COORDINATE_LIMIT || chunk.1.abs() >= START_CACHE_COORDINATE_LIMIT {
            return self.compute_start(chunk);
        }
        let key = (chunk.0 as u32 as u64 & START_CACHE_COORDINATE_MASK) << 31 | (chunk.1 as u32 as u64 & START_CACHE_COORDINATE_MASK);
        let slot = &self.starts[(key.wrapping_mul(START_CACHE_MIX) >> START_CACHE_SHIFT) as usize];
        let stored = slot.load(Ordering::Relaxed);
        if stored & START_CACHE_VALID != 0 && stored & START_CACHE_KEY == key {
            return stored & START_CACHE_RESULT != 0;
        }
        let start = self.compute_start(chunk);
        slot.store(START_CACHE_VALID | key | if start { START_CACHE_RESULT } else { 0 }, Ordering::Relaxed);
        start
    }

    fn compute_start(&self, chunk: (i32, i32)) -> bool {
        let [_, chance, roll] = MersenneTwister::leading_words::<3>(self.chunk_seed(chunk));
        let roll = (roll % MINESHAFT_DISTANCE_RANGE as u32) as i32;
        MersenneTwister::float_from_word(chance) < MINESHAFT_CHANCE && roll < chunk.0.unsigned_abs().max(chunk.1.unsigned_abs()) as i32
    }

    pub fn random(&self, (x, z): (i32, i32)) -> (bool, MersenneTwister) {
        let mut random = MersenneTwister::new(self.chunk_seed((x, z)));
        random.next_u32();
        let chance = random.next_float();
        let roll = random.next_int_bounded(MINESHAFT_DISTANCE_RANGE);
        (chance < MINESHAFT_CHANCE && roll < x.unsigned_abs().max(z.unsigned_abs()) as i32, random)
    }
}

pub fn stronghold_cell(chunk: i32) -> i32 {
    (chunk as f32 / STRONGHOLD_CELL_SIZE as f32).floor() as i32
}

pub fn stronghold_cell_draws(level_seed: u32, (cell_x, cell_z): (i32, i32)) -> Option<((i32, i32), u32)> {
    let centre_x = cell_x * STRONGHOLD_CELL_SIZE + STRONGHOLD_CELL_OFFSET;
    let centre_z = cell_z * STRONGHOLD_CELL_SIZE + STRONGHOLD_CELL_OFFSET;
    if centre_x * centre_x + centre_z * centre_z <= STRONGHOLD_SPAWN_RADIUS_SQUARED {
        return None;
    }
    let seed = centre_z.wrapping_mul(STRONGHOLD_CELL_Z_MULTIPLIER).wrapping_add(centre_x.wrapping_mul(STRONGHOLD_CELL_X_MULTIPLIER)) as u32;
    let mut random = MersenneTwister::new(seed.wrapping_add(level_seed).wrapping_add(STRONGHOLD_SALT));
    let offset_x = (random.next_u32() % STRONGHOLD_SPREAD) as i32;
    let offset_z = random.next_int_bounded(STRONGHOLD_SPREAD as i32);
    let start = (
        cell_x * STRONGHOLD_CELL_SIZE + offset_x + STRONGHOLD_SPREAD_OFFSET,
        cell_z * STRONGHOLD_CELL_SIZE + offset_z + STRONGHOLD_SPREAD_OFFSET,
    );
    Some((start, random.next_u32()))
}

pub fn stronghold_grid_start(level_seed: u32, (x, z): (i32, i32)) -> Option<(i32, i32)> {
    if x * x + z * z <= STRONGHOLD_SPAWN_RADIUS_SQUARED {
        return None;
    }
    let (start, chance) = stronghold_cell_draws(level_seed, (stronghold_cell(x), stronghold_cell(z)))?;
    (raw_to_unit(chance) < STRONGHOLD_CHANCE).then_some(start)
}

fn raw_to_unit(raw: u32) -> f32 {
    (raw as f64 * RAW_UNIT) as f32
}
