use glam::IVec3;

use super::super::blocks::{BlockId, int};
use super::super::noise::NoiseStack;
use super::super::random::RandomSource;
use super::super::terrain::{HEIGHT, MIN_Y};
use super::predicate::BlockPredicate;
use super::region::Region;

pub fn between_inclusive(random: &mut dyn RandomSource, min: i32, max: i32) -> i32 {
    random.next_int_bounded(max - min + 1) + min
}

fn next_int(random: &mut dyn RandomSource, min: i32, max: i32) -> i32 {
    if min >= max { min } else { random.next_int_bounded(max - min + 1) + min }
}

fn pick_weighted<'a, T>(random: &mut dyn RandomSource, entries: &'a [(T, i32)]) -> &'a T {
    let total: i32 = entries.iter().map(|(_, weight)| weight).sum();
    let mut selection = random.next_int_bounded(total);
    for (value, weight) in entries {
        selection -= weight;
        if selection < 0 {
            return value;
        }
    }
    &entries[entries.len() - 1].0
}

pub fn weighted<'a, T>(random: &mut dyn RandomSource, entries: &'a [(T, i32)]) -> &'a T {
    pick_weighted(random, entries)
}

#[derive(Clone)]
pub enum IntProvider {
    Constant(i32),
    Uniform(i32, i32),
    BiasedToBottom(i32, i32),
    Clamped(Box<IntProvider>, i32, i32),
    ClampedNormal { mean: f32, deviation: f32, min: i32, max: i32 },
    Weighted(Vec<(IntProvider, i32)>),
    Trapezoid(i32, i32, i32),
}

impl From<i32> for IntProvider {
    fn from(value: i32) -> Self {
        Self::Constant(value)
    }
}

impl IntProvider {
    pub fn triangle(range: i32) -> Self {
        Self::Trapezoid(-range, range, 0)
    }

    pub fn sample(&self, random: &mut dyn RandomSource) -> i32 {
        match self {
            Self::Constant(value) => *value,
            Self::Uniform(min, max) => between_inclusive(random, *min, *max),
            Self::BiasedToBottom(min, max) => {
                let inner = random.next_int_bounded(max - min + 1) + 1;
                min + random.next_int_bounded(inner)
            }
            Self::Clamped(source, min, max) => source.sample(random).clamp(*min, *max),
            Self::ClampedNormal { mean, deviation, min, max } => (mean + random.next_gaussian() as f32 * deviation).clamp(*min as f32, *max as f32) as i32,
            Self::Weighted(entries) => pick_weighted(random, entries).sample(random),
            Self::Trapezoid(min, max, plateau) => {
                if *plateau == 0 && *max == -*min {
                    return random.next_int_bounded(max + 1) - random.next_int_bounded(max + 1);
                }
                let range = max - min;
                if *plateau == range {
                    return between_inclusive(random, *min, *max);
                }
                let plateau_start = (range - plateau) / 2;
                let plateau_end = range - plateau_start;
                min + between_inclusive(random, 0, plateau_end) + between_inclusive(random, 0, plateau_start)
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum FloatProvider {
    Constant(f32),
    Uniform(f32, f32),
    ClampedNormal { mean: f32, deviation: f32, min: f32, max: f32 },
}

impl FloatProvider {
    pub fn sample(&self, random: &mut dyn RandomSource) -> f32 {
        match *self {
            Self::Constant(value) => value,
            Self::Uniform(min, max) => random.next_float() * (max - min) + min,
            Self::ClampedNormal { mean, deviation, min, max } => (mean + random.next_gaussian() as f32 * deviation).clamp(min, max),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Anchor {
    Absolute(i32),
    AboveBottom(i32),
    BelowTop(i32),
}

pub const BOTTOM: Anchor = Anchor::AboveBottom(0);
pub const TOP: Anchor = Anchor::BelowTop(0);

impl Anchor {
    pub fn resolve(self) -> i32 {
        match self {
            Self::Absolute(y) => y,
            Self::AboveBottom(offset) => MIN_Y + offset,
            Self::BelowTop(offset) => MIN_Y + HEIGHT - 1 - offset,
        }
    }
}

#[derive(Clone, Copy)]
pub enum HeightProvider {
    Uniform(Anchor, Anchor),
    Trapezoid(Anchor, Anchor, i32),
    VeryBiasedToBottom(Anchor, Anchor, i32),
}

impl HeightProvider {
    pub fn sample(&self, random: &mut dyn RandomSource) -> i32 {
        match *self {
            Self::Uniform(min, max) => {
                let (min, max) = (min.resolve(), max.resolve());
                if min > max { min } else { between_inclusive(random, min, max) }
            }
            Self::Trapezoid(min, max, plateau) => {
                let (min, max) = (min.resolve(), max.resolve());
                if min > max {
                    return min;
                }
                let range = max - min;
                if plateau >= range {
                    return between_inclusive(random, min, max);
                }
                let plateau_start = (range - plateau) / 2;
                let plateau_end = range - plateau_start;
                min + between_inclusive(random, 0, plateau_end) + between_inclusive(random, 0, plateau_start)
            }
            Self::VeryBiasedToBottom(min, max, inner) => {
                let (min, max) = (min.resolve(), max.resolve());
                if max - min - inner < 0 {
                    return min;
                }
                let upper = next_int(random, min + inner, max);
                let biased = next_int(random, min, upper - 1);
                next_int(random, min, biased - 1 + inner)
            }
        }
    }
}

pub enum StateProvider {
    Simple(BlockId),
    Weighted(Vec<(BlockId, i32)>),
    RandomizedInt {
        source: Box<StateProvider>,
        property: &'static str,
        values: IntProvider,
    },
    Noise {
        noise: NoiseStack,
        scale: f32,
        states: Vec<BlockId>,
    },
    NoiseThreshold {
        noise: NoiseStack,
        scale: f32,
        threshold: f32,
        high_chance: f32,
        default: BlockId,
        low: Vec<BlockId>,
        high: Vec<BlockId>,
    },
    DualNoise {
        noise: NoiseStack,
        slow_noise: NoiseStack,
        scale: f32,
        slow_scale: f32,
        variety: (i32, i32),
        states: Vec<BlockId>,
    },
    RuleBased {
        fallback: Option<Box<StateProvider>>,
        rules: Vec<(BlockPredicate, StateProvider)>,
    },
}

fn sample_noise(noise: &NoiseStack, pos: IVec3, scale: f64) -> f32 {
    noise.get(pos.x as f64 * scale, pos.y as f64 * scale, pos.z as f64 * scale)
}

impl From<BlockId> for StateProvider {
    fn from(block: BlockId) -> Self {
        Self::Simple(block)
    }
}

impl StateProvider {
    pub fn state(&self, region: &Region, random: &mut dyn RandomSource, pos: IVec3) -> BlockId {
        self.optional_state(region, random, pos).unwrap_or_else(|| region.get(pos))
    }

    pub fn optional_state(&self, region: &Region, random: &mut dyn RandomSource, pos: IVec3) -> Option<BlockId> {
        if let Self::RuleBased { fallback, rules } = self {
            for (predicate, provider) in rules {
                if predicate.test(region, pos) {
                    return Some(provider.state(region, random, pos));
                }
            }
            return fallback.as_ref().map(|provider| provider.state(region, random, pos));
        }
        Some(self.base_state(region, random, pos))
    }

    fn base_state(&self, region: &Region, random: &mut dyn RandomSource, pos: IVec3) -> BlockId {
        match self {
            Self::Simple(block) => *block,
            Self::Weighted(entries) => *pick_weighted(random, entries),
            Self::RandomizedInt { source, property, values } => {
                let base = source.state(region, random, pos);
                let value = values.sample(random);
                region.blocks.with(base, (property, int(value)))
            }
            Self::Noise { noise, scale, states } => pick_by_noise(states, sample_noise(noise, pos, *scale as f64)),
            Self::NoiseThreshold {
                noise,
                scale,
                threshold,
                high_chance,
                default,
                low,
                high,
            } => {
                if sample_noise(noise, pos, *scale as f64) < *threshold {
                    low[random.next_int_bounded(low.len() as i32) as usize]
                } else if random.next_float() < *high_chance {
                    high[random.next_int_bounded(high.len() as i32) as usize]
                } else {
                    *default
                }
            }
            Self::DualNoise {
                noise,
                slow_noise,
                scale,
                slow_scale,
                variety,
                states,
            } => {
                let slow = sample_noise(slow_noise, pos, *slow_scale as f64) as f64;
                let (min, max) = (variety.0 as f64, (variety.1 + 1) as f64);
                let count = (min + ((slow + 1.0) / 2.0).clamp(0.0, 1.0) * (max - min)) as i32;
                let candidates: Vec<BlockId> = (0..count)
                    .map(|i| pick_by_noise(states, sample_noise(slow_noise, pos + IVec3::new(i * 54545, 0, i * 34234), *slow_scale as f64)))
                    .collect();
                pick_by_noise(&candidates, sample_noise(noise, pos, *scale as f64))
            }
            Self::RuleBased { .. } => unreachable!(),
        }
    }
}

fn pick_by_noise(states: &[BlockId], value: f32) -> BlockId {
    let normalized = ((1.0 + value) / 2.0).clamp(0.0, 0.9999);
    states[(normalized * states.len() as f32) as usize]
}
