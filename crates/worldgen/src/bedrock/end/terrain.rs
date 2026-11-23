use crate::shared::noise::legacy::{self, PerlinOctave, SimplexNoise};
use crate::shared::rng::mersenne::MersenneTwister;

pub const HEIGHT: i32 = 128;
pub const ROWS: usize = 33;
pub const CELLS: usize = 3;

const COLUMNS: usize = CELLS * CELLS;
const MAIN_SCALE: [f32; 2] = [17.1103, 4.277_575];
const LIMIT_SCALE: [f32; 2] = [1368.824, 684.412];
const LIMIT_VALUE_SCALE: f32 = 0.001_953_125;
const MAIN_DIVISOR: f32 = 10.0;
const BLEND_OFFSET: f32 = 1.0;
const BLEND_HALF: f32 = 0.5;
const HEIGHT_OFFSET: f32 = -8.0;
const HEIGHT_MIN: f32 = -100.0;
const HEIGHT_MAX: f32 = 80.0;
const ISLAND_CHANCE_ABOVE: f32 = -0.9;
const ISLAND_SCAN: i32 = 12;
const ISLAND_GRID_FROM: i64 = 4096;
const ISLAND_OUTER_FACTOR: f32 = 100.0;
const ISLAND_RING: f32 = 8.0;
const ISLAND_RAISE: f32 = 100.0;
const ISLAND_STEP_A: u32 = 147;
const ISLAND_STEP_B: u32 = 3439;
const ISLAND_STEPS: u32 = 13;
const ISLAND_STEP_BASE: u32 = 9;
const TOP_SLIDE_ROW: usize = 14;
const TOP_SLIDE_SCALE: f32 = 0.015_625;
const TOP_SLIDE_TARGET: f32 = -3000.0;
const BOTTOM_SLIDE_ROW: usize = 8;
const BOTTOM_SLIDE_TARGET: f32 = -30.0;
const BOTTOM_SLIDE_ROWS: f32 = 7.0;
const LAST_ROW_FACTOR: f32 = 0.71875;
const LAST_ROW_BIAS: f32 = -843.75;
const UNIT_STEP: f32 = 0.125;
const SAMPLE_STEP: f32 = 0.25;

pub struct EndNoises {
    lower_limit: Vec<PerlinOctave>,
    upper_limit: Vec<PerlinOctave>,
    main: Vec<PerlinOctave>,
    island: SimplexNoise,
}

pub struct IslandSource {
    step_x: i32,
    step_z: i32,
    factor: f32,
}

impl EndNoises {
    pub fn new(seed: u32) -> Self {
        let mut random = MersenneTwister::new(seed);
        let lower_limit = legacy::perlin_octaves(&mut random, 16);
        let upper_limit = legacy::perlin_octaves(&mut random, 16);
        let main = legacy::perlin_octaves(&mut random, 8);
        let island = island_noise(&mut random);
        Self {
            lower_limit,
            upper_limit,
            main,
            island,
        }
    }

    pub fn island_height(&self, chunk_x: i32, chunk_z: i32, cell_x: i32, cell_z: i32) -> f32 {
        Self::island_height_from(&self.island_sources(chunk_x, chunk_z), chunk_x, chunk_z, cell_x, cell_z)
    }

    pub fn island_sources(&self, chunk_x: i32, chunk_z: i32) -> Vec<IslandSource> {
        let mut sources = Vec::new();
        for step_x in -ISLAND_SCAN..=ISLAND_SCAN {
            let column_x = chunk_x + step_x;
            for step_z in -ISLAND_SCAN..=ISLAND_SCAN {
                let column_z = chunk_z + step_z;
                if i64::from(column_z) * i64::from(column_z) + i64::from(column_x) * i64::from(column_x) <= ISLAND_GRID_FROM
                    || self.island.value(column_x as f32, column_z as f32) >= ISLAND_CHANCE_ABOVE
                {
                    continue;
                }
                let factor = ((column_z.unsigned_abs().wrapping_mul(ISLAND_STEP_A)).wrapping_add(column_x.unsigned_abs().wrapping_mul(ISLAND_STEP_B)) % ISLAND_STEPS + ISLAND_STEP_BASE) as f32;
                sources.push(IslandSource { step_x, step_z, factor });
            }
        }
        sources
    }

    pub fn island_height_from(sources: &[IslandSource], chunk_x: i32, chunk_z: i32, cell_x: i32, cell_z: i32) -> f32 {
        let clamp = |height: f32| height.clamp(HEIGHT_MIN, HEIGHT_MAX);
        let (x, z) = ((cell_x + chunk_x * 2) as f32, (cell_z + chunk_z * 2) as f32);
        let mut height = clamp((z * z + x * x).sqrt() * -ISLAND_RING + ISLAND_RAISE);
        for source in sources {
            let local_x = (cell_x + source.step_x * -2) as f32;
            let local_z = (cell_z - source.step_z * 2) as f32;
            let outer = clamp(ISLAND_OUTER_FACTOR - (local_z * local_z + local_x * local_x).sqrt() * source.factor);
            if outer > height {
                height = outer;
            }
        }
        height
    }

    pub fn cells(&self, chunk_x: i32, chunk_z: i32, sources: &[IslandSource]) -> Vec<f32> {
        let origin = [(chunk_x * 2) as f32, 0.0, (chunk_z * 2) as f32];
        let main = fill_cells(&self.main, origin, MAIN_SCALE);
        let lower = fill_cells(&self.lower_limit, origin, LIMIT_SCALE);
        let upper = fill_cells(&self.upper_limit, origin, LIMIT_SCALE);
        let mut result = vec![0.0f32; COLUMNS * ROWS];
        for column in 0..COLUMNS {
            let height = Self::island_height_from(sources, chunk_x, chunk_z, (column / CELLS) as i32, (column % CELLS) as i32);
            for row in 0..ROWS {
                let index = column * ROWS + row;
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
                result[index] = slide(row, blended, height);
            }
        }
        result
    }
}

fn slide(row: usize, blended: f32, height: f32) -> f32 {
    if row == ROWS - 1 {
        return (height + (blended + HEIGHT_OFFSET)) * LAST_ROW_FACTOR + LAST_ROW_BIAS;
    }
    let value = blended + HEIGHT_OFFSET + height;
    if row > TOP_SLIDE_ROW {
        let t = ((row - TOP_SLIDE_ROW) as f32 * TOP_SLIDE_SCALE).clamp(0.0, 1.0);
        t * TOP_SLIDE_TARGET + (1.0 - t) * value
    } else if row < BOTTOM_SLIDE_ROW {
        let t = (BOTTOM_SLIDE_ROW - row) as f32 / BOTTOM_SLIDE_ROWS;
        t * BOTTOM_SLIDE_TARGET + (1.0 - t) * value
    } else {
        value
    }
}

pub fn fill_stone(cells: &[f32], mut stone: impl FnMut(usize, i32, usize)) {
    let at = |x: usize, z: usize, y: usize| cells[(x * CELLS + z) * ROWS + y];
    for cell_x in 0..2 {
        for cell_z in 0..2 {
            for cell_y in 0..32 {
                let corner_000 = at(cell_x, cell_z, cell_y);
                let corner_001 = at(cell_x, cell_z, cell_y + 1);
                let corner_100 = at(cell_x + 1, cell_z, cell_y);
                let corner_101 = at(cell_x + 1, cell_z, cell_y + 1);
                let corner_010 = at(cell_x, cell_z + 1, cell_y);
                let corner_011 = at(cell_x, cell_z + 1, cell_y + 1);
                let corner_110 = at(cell_x + 1, cell_z + 1, cell_y);
                let corner_111 = at(cell_x + 1, cell_z + 1, cell_y + 1);
                for local_z in 0..8 {
                    let tz = local_z as f32 * UNIT_STEP;
                    let low = (corner_010 - corner_000) * tz + corner_000;
                    let high = (corner_011 - corner_001) * tz + corner_001;
                    for local_x in 0..8 {
                        let tx = local_x as f32 * UNIT_STEP;
                        let bottom = (((corner_110 - corner_100) * tz + corner_100) - low) * tx + low;
                        let step = (tx * ((tz * (corner_111 - corner_101) + corner_101) - high) + high) - bottom;
                        for (local_y, fraction) in [0.0f32, 0.25, 0.5, 0.75].into_iter().enumerate() {
                            if step * fraction + bottom > 0.0 {
                                stone(cell_x * 8 + local_x, (cell_y * 4 + local_y) as i32, cell_z * 8 + local_z);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn column_top(cells: &[f32], x: usize, z: usize) -> i32 {
    (0..HEIGHT as usize).rev().find(|&y| density(cells, x, y, z) > 0.0).map_or(0, |y| y as i32 + 1)
}

fn density(cells: &[f32], x: usize, y: usize, z: usize) -> f32 {
    let (cell_x, local_x) = (x >> 3, x & 7);
    let (cell_z, local_z) = (z >> 3, z & 7);
    let (cell_y, local_y) = (y >> 2, y & 3);
    let tx = local_x as f32 * UNIT_STEP;
    let tz = local_z as f32 * UNIT_STEP;
    let ty = local_y as f32 * SAMPLE_STEP;
    let at = |x: usize, z: usize, y: usize| cells[(x * CELLS + z) * ROWS + y];
    let corner_000 = at(cell_x, cell_z, cell_y);
    let corner_001 = at(cell_x, cell_z, cell_y + 1);
    let corner_100 = at(cell_x + 1, cell_z, cell_y);
    let corner_101 = at(cell_x + 1, cell_z, cell_y + 1);
    let corner_010 = at(cell_x, cell_z + 1, cell_y);
    let corner_011 = at(cell_x, cell_z + 1, cell_y + 1);
    let corner_110 = at(cell_x + 1, cell_z + 1, cell_y);
    let corner_111 = at(cell_x + 1, cell_z + 1, cell_y + 1);
    let low = (corner_010 - corner_000) * tz + corner_000;
    let high = (corner_011 - corner_001) * tz + corner_001;
    let bottom = (((corner_110 - corner_100) * tz + corner_100) - low) * tx + low;
    let step = (tx * ((tz * (corner_111 - corner_101) + corner_101) - high) + high) - bottom;
    step * ty + bottom
}

fn island_noise(random: &mut MersenneTwister) -> SimplexNoise {
    legacy::simplex_octaves(random, 1).remove(0)
}

fn fill_cells(octaves: &[PerlinOctave], origin: [f32; 3], scales: [f32; 2]) -> Vec<f32> {
    legacy::fill_cells::<3, ROWS>(octaves, origin, scales)
}
