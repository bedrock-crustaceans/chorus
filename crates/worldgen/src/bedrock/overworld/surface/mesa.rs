use super::column::Column;
use super::data::{Materials, MesaSettings};
use super::palette::{BROWN, ORANGE, Palette, RED, WHITE, YELLOW};
use crate::bedrock::overworld::noise::NormalNoise;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::rng::seeds::level_seed32;
use crate::shared::rng::xoroshiro::Xoroshiro;

const BAND_COUNT: usize = 192;
const HARD_CLAY: u8 = 16;
const FIRST_STRIPE_GAP: i32 = 5;
const STRIPE_COUNT_RANDOM: i32 = 10;
const STRIPE_COUNT_BASE: i32 = 5;
const STRIPE_LENGTH_RANDOM: i32 = 3;
const LIGHT_BAND_RANDOM: i32 = 7;
const LIGHT_BAND_STEP_RANDOM: i32 = 16;
const LIGHT_BAND_STEP_BASE: usize = 4;
const LIGHT_BAND_LIMIT: i32 = 8;
const BAND_WHITE: u8 = WHITE as u8;
const BAND_LIGHT_GRAY: u8 = 8;
const BAND_ORANGE_TERRACOTTA_ROW: u8 = 1;
const OFFSET_NAME: &str = "minecraft:clay_bands_offset";
const OFFSET_OCTAVE: i32 = -8;
const SURFACE_NAME: &str = "minecraft:badlands_surface";
const SURFACE_OCTAVE: i32 = -6;
const PILLAR_NAME: &str = "minecraft:badlands_pillar";
const PILLAR_OCTAVE: i32 = -2;
const ROOF_NAME: &str = "minecraft:badlands_pillar_roof";
const ROOF_OCTAVE: i32 = -8;
const OFFSET_SCALE: f32 = 4.0;
const SURFACE_SCALE: f32 = 8.25;
const PILLAR_INPUT: f32 = 0.2;
const PILLAR_SCALE: f32 = 15.0;
const ROOF_INPUT: f32 = 0.75;
const ROOF_SCALE: f32 = 1.5;
const ROOF_HEIGHT_SCALE: f32 = 50.0;
const PILLAR_CURVE: f32 = 2.5;
const ROOF_CEILING_OFFSET: f32 = 24.0;
const PILLAR_BASE: f32 = 64.0;
const COSINE_PERIOD: f32 = std::f32::consts::PI;
const DEPTH_DIVISOR: f32 = 3.0;
const RUN_DEPTH_BASE: f32 = 3.0;
const RUN_DEPTH_RANDOM: f32 = 0.25;
const MAX_COUNT: i32 = 15;
const SAND_BAND_HEIGHT: i32 = 10;
const FOREST_BAND_HEIGHT: i32 = 33;
const BAND_CEILING: i32 = 192;
const SHALLOW_BELOW_SEA: i32 = 1;
const BAND_FLOOR_BELOW_SEA: i32 = 4;
const SEA_SURFACE_ABOVE: i32 = 1;

pub struct MesaColumn<'a> {
    pub x: i32,
    pub z: i32,
    pub lower_bound: i32,
    pub sea_level: i32,
    pub depth: f32,
    pub materials: Materials,
    pub settings: &'a MesaSettings,
}

pub struct Mesa {
    clay_bands_offset: NormalNoise,
    surface: NormalNoise,
    pillar: NormalNoise,
    roof: NormalNoise,
    bands: [u8; BAND_COUNT],
}

fn fill_stripe(bands: &mut [u8; BAND_COUNT], random: &mut MersenneTwister, color: usize, extra_length: i32) {
    let count = random.next_int_bounded(STRIPE_COUNT_RANDOM);
    for _ in 0..(count + STRIPE_COUNT_BASE).max(0) + 1 {
        let length = random.next_int_bounded(STRIPE_LENGTH_RANDOM) + extra_length;
        let start = random.next_int_bounded(BAND_COUNT as i32);
        if length < 0 {
            continue;
        }
        let last = length.min(BAND_COUNT as i32 - 1 - start);
        for offset in 0..=last {
            bands[start as usize + offset as usize] = color as u8;
        }
    }
}

fn generate_bands(seed: u32) -> [u8; BAND_COUNT] {
    let mut random = MersenneTwister::new(seed);
    let mut bands = [HARD_CLAY; BAND_COUNT];
    let mut index = 0i32;
    loop {
        let gap = random.next_int_bounded(FIRST_STRIPE_GAP);
        let position = gap + index + 1;
        if position >= BAND_COUNT as i32 {
            break;
        }
        bands[position as usize] = BAND_ORANGE_TERRACOTTA_ROW;
        index = gap + index + 2;
        if position == BAND_COUNT as i32 - 1 {
            break;
        }
    }
    fill_stripe(&mut bands, &mut random, YELLOW, 0);
    fill_stripe(&mut bands, &mut random, BROWN, 1);
    fill_stripe(&mut bands, &mut random, RED, 0);
    let limit = random.next_int_bounded(LIGHT_BAND_RANDOM);
    let mut position = 0usize;
    let mut iteration = 0;
    loop {
        bands[position] = BAND_WHITE;
        if position >= 2 {
            if random.next_boolean() {
                bands[position - 1] = BAND_LIGHT_GRAY;
            }
            if position < BAND_COUNT - 1 && random.next_boolean() {
                bands[position + 1] = BAND_LIGHT_GRAY;
            }
        } else if random.next_boolean() {
            bands[position + 1] = BAND_LIGHT_GRAY;
        }
        let step = random.next_int_bounded(LIGHT_BAND_STEP_RANDOM) as usize;
        if limit + LIGHT_BAND_LIMIT <= iteration {
            break;
        }
        iteration += 1;
        position += step + LIGHT_BAND_STEP_BASE;
        if position >= BAND_COUNT {
            break;
        }
    }
    bands
}

impl Mesa {
    pub fn new(seed: i64) -> Self {
        let factory = Xoroshiro::new(level_seed32(seed) as i64).fork_positional();
        Self {
            clay_bands_offset: NormalNoise::new(&mut factory.hash_of(OFFSET_NAME), OFFSET_OCTAVE, &[1.0]),
            surface: NormalNoise::new(&mut factory.hash_of(SURFACE_NAME), SURFACE_OCTAVE, &[1.0; 3]),
            pillar: NormalNoise::new(&mut factory.hash_of(PILLAR_NAME), PILLAR_OCTAVE, &[1.0; 4]),
            roof: NormalNoise::new(&mut factory.hash_of(ROOF_NAME), ROOF_OCTAVE, &[1.0]),
            bands: generate_bands(level_seed32(seed) as u32),
        }
    }

    fn pillar_top(&self, x: f32, z: f32) -> f32 {
        let surface = (self.surface.value(x, 0.0, z) * SURFACE_SCALE).abs();
        let pillar = self.pillar.value(x * PILLAR_INPUT, 0.0, z * PILLAR_INPUT) * PILLAR_SCALE;
        let depth = if pillar <= surface { pillar } else { surface };
        if 0.0 < depth {
            let roof = (self.roof.value(x * ROOF_INPUT, 0.0, z * ROOF_INPUT) * ROOF_SCALE).abs() * ROOF_HEIGHT_SCALE;
            let mut top = depth * depth * PILLAR_CURVE;
            let ceiling = roof.ceil() + ROOF_CEILING_OFFSET;
            if ceiling <= top {
                top = ceiling;
            }
            top + PILLAR_BASE
        } else {
            0.0
        }
    }

    pub fn build(&self, palette: &Palette, column: &mut Column, spec: &MesaColumn, random: &mut MersenneTwister) {
        let (fx, fz) = (spec.x as f32, spec.z as f32);
        let pillar_top = if spec.settings.bryce_pillars { self.pillar_top(fx, fz) } else { 0.0 };
        let depth_third = spec.depth / DEPTH_DIVISOR;
        let random_value = random.next_float();
        let cosine = (COSINE_PERIOD * depth_third).cos();
        let height = column.height();
        let lower = spec.lower_bound - column.min_y();
        let sea = spec.sea_level - column.min_y();
        for y in (lower + 1..=height).rev() {
            let block = column.get_local(y - 1);
            if palette.is_liquid(block) {
                return;
            }
            if block != palette.air {
                break;
            }
        }
        let run_depth = (random_value * RUN_DEPTH_RANDOM + (depth_third + RUN_DEPTH_BASE)) as i32;
        let band_offset = (self.clay_bands_offset.value(fx, 0.0, fz) * OFFSET_SCALE).round() as i32;
        let band_color = |y: i32| -> u8 { self.bands[(y + BAND_COUNT as i32 + band_offset).rem_euclid(BAND_COUNT as i32) as usize] };
        let colored = |band: u8| if band < HARD_CLAY { palette.terracotta[band as usize] } else { spec.settings.hard_clay };
        let pillar_floor = pillar_top as i32 - (pillar_top < (pillar_top as i32) as f32) as i32 - column.min_y();
        let mut run = -1;
        let mut count = 0;
        let mut mid_variable = spec.materials.mid;
        let mut in_sand = false;
        let mut stop = false;
        for y in (lower..height).rev() {
            let mut block = column.get_local(y);
            if block == palette.bedrock {
                return;
            }
            let was_air = block == palette.air;
            let filled = y <= pillar_floor && !stop && was_air;
            if filled {
                block = palette.stone;
                column.set_local(y, block);
                stop = false;
            } else {
                stop = stop || !was_air;
            }
            if count >= MAX_COUNT {
                continue;
            }
            if block == palette.air {
                run = -1;
            } else if block == palette.stone {
                if run == -1 {
                    let candidate = if sea + SEA_SURFACE_ABOVE < y { mid_variable } else { spec.materials.mid };
                    if sea - BAND_FLOOR_BELOW_SEA <= y {
                        mid_variable = candidate;
                    }
                    if run_depth < 1 {
                        mid_variable = spec.materials.foundation;
                    }
                    run = (y - sea).max(0) + run_depth;
                    in_sand = false;
                    let placed = if y < sea - SHALLOW_BELOW_SEA {
                        if mid_variable == palette.terracotta[WHITE] { palette.terracotta[ORANGE] } else { mid_variable }
                    } else if !spec.settings.has_forest || y <= sea + FOREST_BAND_HEIGHT + run_depth * 2 {
                        if y <= sea + SAND_BAND_HEIGHT + run_depth {
                            in_sand = true;
                            spec.materials.top
                        } else {
                            let in_bands = y <= sea + BAND_CEILING && sea < y;
                            let band = if in_bands && cosine <= 0.0 {
                                band_color(y)
                            } else if in_bands {
                                HARD_CLAY
                            } else {
                                BAND_ORANGE_TERRACOTTA_ROW
                            };
                            colored(band)
                        }
                    } else if cosine <= 0.0 {
                        palette.grass_block
                    } else {
                        palette.coarse_dirt
                    };
                    column.set_local(y, placed);
                } else if run > 0 {
                    run -= 1;
                    column.set_local(y, if in_sand { palette.terracotta[ORANGE] } else { colored(band_color(y)) });
                }
                if !filled {
                    count += 1;
                }
            }
        }
    }
}
