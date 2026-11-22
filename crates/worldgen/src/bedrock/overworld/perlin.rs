use super::noise::{fade, gradient, next_double};
use crate::shared::noise::lerp;
use crate::shared::rng::random::RandomSource;
use crate::shared::rng::xoroshiro::{PositionalFactory, Xoroshiro};

const PERMUTATION_SIZE: usize = 256;
const ORIGIN_SCALE: f32 = 256.0;
const MIN_LIMIT_LEVELS: usize = 16;
const MAX_LIMIT_LEVELS: usize = 16;
const MAIN_LEVELS: usize = 8;
const LIMIT_SCALE: f32 = 684.412;
const MAIN_SCALE_XZ: f32 = 8.55515;
const MAIN_SCALE_Y: f32 = 4.277575;
const FREQUENCY_STEP: f32 = 0.5;
const TERRAIN_NAME: &str = "minecraft:terrain";

pub const GRID_COLUMNS: usize = 5;

struct PerlinLevel {
    origin: [f32; 3],
    permutation: [i32; PERMUTATION_SIZE * 2],
}

impl PerlinLevel {
    fn new(random: &mut Xoroshiro) -> Self {
        let origin = [0, 1, 2].map(|_| next_double(random) as f32 * ORIGIN_SCALE);
        let mut permutation = [0i32; PERMUTATION_SIZE * 2];
        for (i, slot) in permutation.iter_mut().take(PERMUTATION_SIZE).enumerate() {
            *slot = i as i32;
        }
        for i in 0..PERMUTATION_SIZE {
            let j = random.next_int_bounded((PERMUTATION_SIZE - i) as i32) as usize + i;
            permutation.swap(i, j);
        }
        for i in 0..PERMUTATION_SIZE {
            permutation[i + PERMUTATION_SIZE] = permutation[i];
        }
        Self { origin, permutation }
    }

    fn read_area(&self, power: f32, out: &mut [f32], position: [f32; 3], scale: [f32; 3], grid_levels: usize) {
        let inverse = 1.0 / power;
        let p = &self.permutation;
        let hash = |index: i32| p[index as usize] as usize;
        let y_start = 0.0 * scale[1] + self.origin[1];
        let mut cached_y = -1i32;
        let mut cache = [0.0f32; 4];
        let mut index = 0;
        for i in 0..GRID_COLUMNS {
            let fx = (i as f32 + position[0]) * scale[0] + self.origin[0];
            let floor_x = fx.floor();
            let x = floor_x as i32 & 255;
            let u = fx - floor_x;
            let fu = fade(u);
            for k in 0..GRID_COLUMNS {
                let fz = (k as f32 + position[2]) * scale[2] + self.origin[2];
                let floor_z = fz.floor();
                let z = floor_z as i32 & 255;
                let w = fz - floor_z;
                for j in 0..grid_levels {
                    let fy = (j as f32 + position[1]) * scale[1] + self.origin[1];
                    let floor_y = fy.floor();
                    let y = floor_y as i32 & 255;
                    let t = fy - floor_y;
                    let special = y_start as i32 == fy as i32;
                    if special || y != cached_y {
                        let gy = if special { y_start - (y_start as i32) as f32 } else { t - (t / scale[1]).floor() * scale[1] };
                        let a = p[x as usize] + y;
                        let b = p[x as usize + 1] + y;
                        cache = [
                            lerp(fu, gradient(hash(hash(a) as i32 + z), u, gy, w), gradient(hash(hash(b) as i32 + z), u - 1.0, gy, w)),
                            lerp(fu, gradient(hash(hash(a + 1) as i32 + z), u, gy - 1.0, w), gradient(hash(hash(b + 1) as i32 + z), u - 1.0, gy - 1.0, w)),
                            lerp(fu, gradient(hash(hash(a) as i32 + z + 1), u, gy, w - 1.0), gradient(hash(hash(b) as i32 + z + 1), u - 1.0, gy, w - 1.0)),
                            lerp(
                                fu,
                                gradient(hash(hash(a + 1) as i32 + z + 1), u, gy - 1.0, w - 1.0),
                                gradient(hash(hash(b + 1) as i32 + z + 1), u - 1.0, gy - 1.0, w - 1.0),
                            ),
                        ];
                        cached_y = y;
                    }
                    let ft = fade(t);
                    let v0 = lerp(ft, cache[0], cache[1]);
                    let v1 = lerp(ft, cache[2], cache[3]);
                    out[index] += lerp(fade(w), v0, v1) * inverse;
                    index += 1;
                }
            }
        }
    }
}

pub struct TerrainBuffers {
    pub min_limit: Vec<f32>,
    pub max_limit: Vec<f32>,
    pub main: Vec<f32>,
}

impl TerrainBuffers {
    pub fn new() -> Self {
        Self {
            min_limit: Vec::new(),
            max_limit: Vec::new(),
            main: Vec::new(),
        }
    }
}

pub struct LegacyBlendedNoise {
    min_limit: Vec<PerlinLevel>,
    max_limit: Vec<PerlinLevel>,
    main: Vec<PerlinLevel>,
}

impl LegacyBlendedNoise {
    pub fn new(factory: &PositionalFactory) -> Self {
        let mut random = factory.hash_of(TERRAIN_NAME);
        let mut levels = |count: usize| (0..count).map(|_| PerlinLevel::new(&mut random)).collect::<Vec<_>>();
        let min_limit = levels(MIN_LIMIT_LEVELS);
        let max_limit = levels(MAX_LIMIT_LEVELS);
        let main = levels(MAIN_LEVELS);
        Self { min_limit, max_limit, main }
    }

    pub fn fill(&self, buffers: &mut TerrainBuffers, position: [f32; 3], grid_levels: usize) {
        let size = GRID_COLUMNS * GRID_COLUMNS * grid_levels;
        for buffer in [&mut buffers.min_limit, &mut buffers.max_limit, &mut buffers.main] {
            buffer.resize(size, 0.0);
        }
        Self::accumulate(&self.min_limit, &mut buffers.min_limit, position, [LIMIT_SCALE, LIMIT_SCALE, LIMIT_SCALE], grid_levels);
        Self::accumulate(&self.max_limit, &mut buffers.max_limit, position, [LIMIT_SCALE, LIMIT_SCALE, LIMIT_SCALE], grid_levels);
        Self::accumulate(&self.main, &mut buffers.main, position, [MAIN_SCALE_XZ, MAIN_SCALE_Y, MAIN_SCALE_XZ], grid_levels);
    }

    fn accumulate(levels: &[PerlinLevel], out: &mut [f32], position: [f32; 3], scale: [f32; 3], grid_levels: usize) {
        out.fill(0.0);
        let mut frequency = 1.0f32;
        for level in levels {
            level.read_area(frequency, out, position, scale.map(|axis| axis * frequency), grid_levels);
            frequency *= FREQUENCY_STEP;
        }
    }
}
