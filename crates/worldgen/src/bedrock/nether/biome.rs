use crate::shared::noise::legacy::LegacyPerlin;
use crate::shared::rng::mersenne::MersenneTwister;

pub use crate::shared::biome::pack_ids::{BASALT_DELTAS, CRIMSON_FOREST, HELL, SOULSAND_VALLEY, WARPED_FOREST};

pub const BIOMES: [i32; 5] = [HELL, SOULSAND_VALLEY, CRIMSON_FOREST, WARPED_FOREST, BASALT_DELTAS];

const RULES: [(i32, [f32; 5]); 5] = [
    (HELL, [0.0, 0.0, 0.0, 0.0, 0.0]),
    (SOULSAND_VALLEY, [0.0, -0.5, 0.0, 0.0, 0.0]),
    (CRIMSON_FOREST, [0.4, 0.0, 0.0, 0.0, 0.0]),
    (WARPED_FOREST, [0.0, 0.5, 0.0, 0.0, 0.375]),
    (BASALT_DELTAS, [-0.5, 0.0, 0.0, 0.0, 0.175]),
];

const ZOOM_SALT: i64 = -0x7945_4928_9d90_b428;
const LCG_MULTIPLIER: i64 = 0x5851_f42d_4c95_7f2d;
const LCG_ADDEND: i64 = 0x1405_7b7e_f767_814f;
const CLIMATE_OCTAVES: usize = 2;
const CLIMATE_FREQUENCY: f32 = 0.015625;
const CLIMATE_SECOND_SCALE: f32 = f32::from_bits(0x3f82_51fb);
const CLIMATE_FACTOR: f32 = 0.166_666_67 / ((1.0 / 3.0 + 1.0) * 0.1);
const JITTER_SHIFT: i64 = 24;
const JITTER_MASK: i64 = 0x3ff;
const JITTER_STEP: f32 = 0.000_976_562_5;
const JITTER_BIAS: f32 = -0.5;
const JITTER_SCALE: f32 = 3.6;
const JITTER_CORNERS: [(i32, i32); 4] = [(0, 0), (1, 0), (0, 1), (1, 1)];
const ZOOM_ORIGIN: i32 = 2;

struct ClimateLayer {
    first: LegacyPerlin,
    second: LegacyPerlin,
}

impl ClimateLayer {
    fn new(seed: i64, index: u32) -> Self {
        let mut random = MersenneTwister::new((seed as u32).wrapping_add(index));
        let first = LegacyPerlin::new(&mut random, CLIMATE_OCTAVES);
        let second = LegacyPerlin::new(&mut random, CLIMATE_OCTAVES);
        Self { first, second }
    }

    fn value(&self, quart_x: i32, quart_z: i32) -> f32 {
        let (x, z) = (quart_x as f32, quart_z as f32);
        let first = self.first.value([x * CLIMATE_FREQUENCY, 0.0, z * CLIMATE_FREQUENCY]) * self.first.normalisation;
        let second = self.second.value([x * CLIMATE_SECOND_SCALE * CLIMATE_FREQUENCY, 0.0, z * CLIMATE_SECOND_SCALE * CLIMATE_FREQUENCY]) * self.second.normalisation;
        (second + first) * CLIMATE_FACTOR
    }
}

pub struct NetherBiomes {
    climate: [ClimateLayer; 4],
    zoom_seed: i64,
}

fn mix(state: i64, add: i64) -> i64 {
    state.wrapping_mul(state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_ADDEND)).wrapping_add(add)
}

impl NetherBiomes {
    pub fn new(seed: i64) -> Self {
        let zoom_seed = mix(mix(mix(seed, ZOOM_SALT), ZOOM_SALT), ZOOM_SALT);
        Self {
            climate: std::array::from_fn(|index| ClimateLayer::new(seed, index as u32)),
            zoom_seed,
        }
    }

    pub fn quart_biome(&self, quart_x: i32, quart_z: i32) -> i32 {
        let climate = self.climate.each_ref().map(|layer| layer.value(quart_x, quart_z));
        let mut best = (HELL, f32::MAX);
        for (biome, [temperature, humidity, altitude, weirdness, weight]) in RULES {
            let (dt, dh, da, dw) = (temperature - climate[0], humidity - climate[1], altitude - climate[2], weirdness - climate[3]);
            let distance = dt * dt + dh * dh + da * da + dw * dw + weight * weight;
            if distance < best.1 {
                best = (biome, distance);
            }
        }
        best.0
    }

    fn jitter(&self, x: i32, z: i32) -> [f32; 2] {
        let (x, z) = (i64::from(x), i64::from(z));
        let mut state = mix(mix(mix(mix(self.zoom_seed, x), z), x), z);
        std::array::from_fn(|_| {
            let draw = ((state >> JITTER_SHIFT) & JITTER_MASK) as f32;
            state = mix(state, self.zoom_seed);
            (draw * JITTER_STEP + JITTER_BIAS) * JITTER_SCALE
        })
    }

    fn zoomed_quart(&self, x: i32, z: i32) -> (i32, i32) {
        let (x, z) = (x - ZOOM_ORIGIN, z - ZOOM_ORIGIN);
        let (quart_x, quart_z) = (x >> 2, z >> 2);
        let (offset_x, offset_z) = ((x & 3) as f32, (z & 3) as f32);
        let distances = JITTER_CORNERS.map(|(corner_x, corner_z)| {
            let [dx, dz] = self.jitter((quart_x + corner_x) * 4, (quart_z + corner_z) * 4);
            let (dx, dz) = (dx + 4.0 * corner_x as f32 - offset_x, dz + 4.0 * corner_z as f32 - offset_z);
            dx * dx + dz * dz
        });
        let pick = (0..3).find(|&index| (0..4).all(|other| other == index || distances[index] < distances[other])).unwrap_or(3);
        let (corner_x, corner_z) = JITTER_CORNERS[pick];
        (quart_x + corner_x, quart_z + corner_z)
    }

    pub fn biome(&self, x: i32, z: i32) -> i32 {
        let (quart_x, quart_z) = self.zoomed_quart(x, z);
        self.quart_biome(quart_x, quart_z)
    }

    pub fn chunk_biomes(&self, chunk_x: i32, chunk_z: i32) -> [i32; 256] {
        const SPAN: usize = 6;
        let (base_x, base_z) = (chunk_x * 4 - 1, chunk_z * 4 - 1);
        let mut memo = [[None; SPAN]; SPAN];
        let mut biomes = [0i32; 256];
        for x in 0..16 {
            for z in 0..16 {
                let (quart_x, quart_z) = self.zoomed_quart(chunk_x * 16 + x, chunk_z * 16 + z);
                let slot = &mut memo[(quart_x - base_x) as usize][(quart_z - base_z) as usize];
                biomes[(x * 16 + z) as usize] = *slot.get_or_insert_with(|| self.quart_biome(quart_x, quart_z));
            }
        }
        biomes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_biomes_match_per_column_lookup() {
        let biomes = NetherBiomes::new(899197117575234806);
        for (chunk_x, chunk_z) in [(0, 0), (-1, -1), (-7, 12), (31, -40), (5, 5)] {
            let table = biomes.chunk_biomes(chunk_x, chunk_z);
            for x in 0..16 {
                for z in 0..16 {
                    assert_eq!(table[(x * 16 + z) as usize], biomes.biome(chunk_x * 16 + x, chunk_z * 16 + z));
                }
            }
        }
    }
}
