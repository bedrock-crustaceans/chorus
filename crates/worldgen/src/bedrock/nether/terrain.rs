use super::noise;
use crate::shared::math::sine;
use crate::shared::noise::legacy;
use crate::shared::rng::mersenne::MersenneTwister;

pub const TERRAIN_HEIGHT: i32 = 128;
pub const SEA_LEVEL: i32 = 32;
pub const CELL_ROWS: usize = 17;
pub const CELL_SIDE: usize = 5;

const MAIN_SCALE: [f32; 2] = [f32::from_bits(0x4108_e1e5), f32::from_bits(0x4208_e1e4)];
const LIMIT_XZ: f32 = f32::from_bits(0x442b_1a5e);
const LIMIT_Y: f32 = f32::from_bits(0x4500_53c6);
const LIMIT_VALUE_SCALE: f32 = 0.001_953_125;
const MAIN_DIVISOR: f32 = 10.0;
const BLEND_OFFSET: f32 = 1.0;
const BLEND_HALF: f32 = 0.5;
const OCTAVE_DRAWS: usize = 3 + 256;
const UNUSED_OCTAVES: usize = 4;
const UNUSED_TWO: usize = 10 + 16;
const OFFSET_PI: f32 = std::f32::consts::PI;
const OFFSET_FACTOR: f32 = 6.0;
const OFFSET_ROWS: f32 = 17.0;
const OFFSET_GROWTH: f32 = -10.0;
const OFFSET_EDGE: f32 = 4.0;
const OFFSET_MIDDLE: i32 = 8;
const HALF_ROWS: i32 = 16;
const SLIDE_14_SCALE: f32 = f32::from_bits(0x3f2a_aaaa);
const SLIDE_14_BIAS: f32 = f32::from_bits(0xc055_5556);
const SLIDE_15_SCALE: f32 = f32::from_bits(0x3eaa_aaaa);
const SLIDE_15_BIAS: f32 = f32::from_bits(0xc0d5_5556);
const CELL_STEP: f32 = 0.25;
const ROW_STEP: f32 = 0.125;

#[derive(Clone, Copy)]
pub enum Cell {
    Stone,
    Water,
    Air,
}

pub struct NetherNoises {
    min_limit: Vec<legacy::PerlinOctave>,
    max_limit: Vec<legacy::PerlinOctave>,
    main: Vec<legacy::PerlinOctave>,
    pub depth: Vec<legacy::SimplexNoise>,
    pub adjustment: Vec<legacy::SimplexNoise>,
}

impl NetherNoises {
    pub fn new(seed: u32) -> Self {
        let mut random = MersenneTwister::new(seed);
        let min_limit = legacy::perlin_octaves(&mut random, 16);
        let max_limit = legacy::perlin_octaves(&mut random, 16);
        let main = legacy::perlin_octaves(&mut random, 8);
        random.skip(UNUSED_OCTAVES * OCTAVE_DRAWS);
        let depth = legacy::simplex_octaves(&mut random, 4);
        random.skip(UNUSED_TWO * OCTAVE_DRAWS);
        let adjustment = legacy::simplex_octaves(&mut random, 3);
        Self {
            min_limit,
            max_limit,
            main,
            depth,
            adjustment,
        }
    }

    pub fn cells(&self, chunk_x: i32, chunk_z: i32) -> Vec<f32> {
        let origin = [(chunk_x * 4) as f32, 0.0, (chunk_z * 4) as f32];
        let main = legacy::fill_cells::<CELL_SIDE, CELL_ROWS>(&self.main, origin, MAIN_SCALE);
        let limits = [LIMIT_XZ, LIMIT_Y];
        let lower = legacy::fill_cells::<CELL_SIDE, CELL_ROWS>(&self.min_limit, origin, limits);
        let upper = legacy::fill_cells::<CELL_SIDE, CELL_ROWS>(&self.max_limit, origin, limits);
        let offsets = y_offsets();
        (0..main.len())
            .map(|index| {
                let low = lower[index] * LIMIT_VALUE_SCALE;
                let high = upper[index] * LIMIT_VALUE_SCALE;
                let blend = (main[index] / MAIN_DIVISOR + BLEND_OFFSET) * BLEND_HALF;
                let blended = if blend < 0.0 {
                    low
                } else if blend <= 1.0 {
                    (high - low) * blend + low
                } else {
                    high
                };
                top_slide(blended - offsets[index % CELL_ROWS], index % CELL_ROWS)
            })
            .collect()
    }

    pub fn fill_shape(&self, chunk_x: i32, chunk_z: i32, mut set: impl FnMut(usize, i32, usize, Cell)) {
        let cells = self.cells(chunk_x, chunk_z);
        let at = |x: usize, z: usize, y: usize| cells[(x * CELL_SIDE + z) * CELL_ROWS + y];
        for cell_x in 0..4 {
            for cell_z in 0..4 {
                for cell_y in 0..CELL_ROWS - 1 {
                    let low_000 = at(cell_x, cell_z, cell_y);
                    let low_100 = at(cell_x + 1, cell_z, cell_y);
                    let low_010 = at(cell_x, cell_z + 1, cell_y);
                    let low_110 = at(cell_x + 1, cell_z + 1, cell_y);
                    let high_001 = at(cell_x, cell_z, cell_y + 1);
                    let high_101 = at(cell_x + 1, cell_z, cell_y + 1);
                    let high_011 = at(cell_x, cell_z + 1, cell_y + 1);
                    let high_111 = at(cell_x + 1, cell_z + 1, cell_y + 1);
                    for local_z in 0..4 {
                        let tz = local_z as f32 * CELL_STEP;
                        let lower_z = (low_010 - low_000) * tz + low_000;
                        let upper_z = (high_011 - high_001) * tz + high_001;
                        for local_x in 0..4 {
                            let tx = local_x as f32 * CELL_STEP;
                            let lower = ((low_110 - low_100) * tz + low_100 - lower_z) * tx + lower_z;
                            let step = (tx * ((high_111 - high_101) * tz + high_101 - upper_z) + upper_z) - lower;
                            for local_y in 0..8 {
                                let y = (cell_y * 8 + local_y) as i32;
                                let cell = if step * (local_y as f32 * ROW_STEP) + lower > 0.0 {
                                    Cell::Stone
                                } else if y < SEA_LEVEL {
                                    Cell::Water
                                } else {
                                    Cell::Air
                                };
                                set(cell_x * 4 + local_x, y, cell_z * 4 + local_z, cell);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn depth_noise(&self, x: i32, z: i32, frequency: f32) -> f32 {
        noise::depth(&self.depth, x, z, frequency)
    }

    pub fn adjustment_noise(&self, x: i32, z: i32, scale: f32) -> f32 {
        noise::adjustment(&self.adjustment, x, z, scale)
    }
}

fn y_offsets() -> [f32; CELL_ROWS] {
    std::array::from_fn(|y| {
        let row = y as f32;
        let edge = if y as i32 > OFFSET_MIDDLE { HALF_ROWS as f32 - row } else { row };
        let mut offset = sine::cos(row * OFFSET_PI * OFFSET_FACTOR / OFFSET_ROWS);
        offset += offset;
        if edge < OFFSET_EDGE {
            let depth = OFFSET_EDGE - edge;
            offset += depth * depth * depth * OFFSET_GROWTH;
        }
        offset
    })
}

fn top_slide(density: f32, row: usize) -> f32 {
    match row {
        14 => density * SLIDE_14_SCALE + SLIDE_14_BIAS,
        15 => density * SLIDE_15_SCALE + SLIDE_15_BIAS,
        16 => density * 0.0 + -10.0,
        _ => density,
    }
}
