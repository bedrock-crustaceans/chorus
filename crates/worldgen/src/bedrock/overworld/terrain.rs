use super::noise::NormalNoise;
use super::perlin::{GRID_COLUMNS, LegacyBlendedNoise, TerrainBuffers};
use crate::shared::math::spline::{Spline, SplineInput, overworld_factor, overworld_jaggedness, overworld_offset};
use crate::shared::rng::xoroshiro::PositionalFactory;
use crate::shared::terrain::{ClimateScale, OverworldSettings};
use chorus_level::dimension_definition::DimensionDefinition;

const CELL_HEIGHT: i32 = 8;
const CELL_WIDTH: i32 = 4;
const SLIDE_TOP_CELLS: i32 = 8;
const MIN_CELL_Y: f32 = -8.0;
const CAVE_BOOST: f32 = 128.0;
const QUANTISE_SCALE: f64 = 10000.0;
const SHIFT_SCALE: f32 = 4.0;
const DEPTH_PER_BLOCK: f32 = -0.0078125;
const PEAK_OFFSET: f32 = -0.6666667;
const VALLEY_OFFSET: f32 = -0.33333334;
const PEAK_SCALE: f32 = -3.0;
const LIMIT_FACTOR: f32 = 0.001953125;
const MAIN_DIVISOR: f32 = 10.0;
const POSITIVE_DENSITY_SCALE: f32 = 4.0;
const JAGGED_INPUT_SCALE: f32 = 1500.0;
const JAGGED_NEGATIVE_FACTOR: f32 = 0.5;
pub const SURFACE_TOP_MARGIN: i32 = 72;
const SURFACE_BOTTOM: f32 = -90.0;
const SURFACE_THRESHOLD: f32 = 50.0;
pub const SURFACE_NOT_FOUND: i32 = 0x7fff;
const SLIDE_TOP_FLOOR: f32 = -10.0;
const SLIDE_TOP_OFFSET: f32 = 10.0;
const SLIDE_BOTTOM_OFFSET: f32 = -15.0;
const SLIDE_BOTTOM_TARGET: f32 = 15.0;
const CAVE_SKIP_BELOW: f32 = -150.0;
const CAVE_CHEESE_BELOW: f32 = 200.0;
const THREE_D_THICKNESS_RANGE: f32 = 0.023000002;
const THREE_D_THICKNESS_MIN: f32 = 0.065;
const TWO_D_THICKNESS_RANGE: f32 = 0.6999999;
const TWO_D_THICKNESS_MIN: f32 = 0.6;
const ELEVATION_EPSILON: f32 = 1.1920929e-07;
const PILLAR_THRESHOLD: f32 = 0.03;
const TUNNEL_BOOST: f32 = 5.0;
const ROUGHNESS_THRESHOLD: f32 = 0.4;
const ROUGHNESS_MODULATOR_SCALE: f32 = 0.1;
const ENTRANCE_HORIZONTAL_SCALE: f32 = 0.75;
const ENTRANCE_VERTICAL_SCALE: f32 = 0.5;
const ENTRANCE_BIAS: f32 = 0.37;
const ENTRANCE_FADE_START: f32 = 10.0;
const ENTRANCE_FADE_SPAN: f32 = 40.0;
const ENTRANCE_FADE: f32 = 0.3;
const CHEESE_Y_SCALE: f32 = 0.6666667;
const CHEESE_BIAS: f32 = 0.27;
const CHEESE_FADE_SCALE: f32 = 1.28;
const CHEESE_FADE: f32 = 0.5;
const LAYER_Y_SCALE: f32 = 8.0;
const LAYER_WEIGHT: f32 = 4.0;
const ELEVATION_MAX: f32 = 8.0;
const ELEVATION_SLOPE: f32 = -0.125;
const RIDGE_THICKNESS_FACTOR: f32 = -0.083;
const PILLAR_RARENESS_SCALE: f32 = 2.0;
const PILLAR_THICKNESS_SCALE: f32 = 1.1;
const PILLAR_HORIZONTAL_SCALE: f32 = 25.0;
const PILLAR_VERTICAL_SCALE: f32 = 0.3;

pub struct CaveNoises {
    pillar: NormalNoise,
    pillar_rareness: NormalNoise,
    pillar_thickness: NormalNoise,
    spaghetti_2d: NormalNoise,
    spaghetti_2d_elevation: NormalNoise,
    spaghetti_2d_modulator: NormalNoise,
    spaghetti_2d_thickness: NormalNoise,
    spaghetti_3d_1: NormalNoise,
    spaghetti_3d_2: NormalNoise,
    spaghetti_3d_rarity: NormalNoise,
    spaghetti_3d_thickness: NormalNoise,
    spaghetti_roughness: NormalNoise,
    spaghetti_roughness_modulator: NormalNoise,
    cave_entrance: NormalNoise,
    cave_layer: NormalNoise,
    cave_cheese: NormalNoise,
}

pub(super) fn clamp_unit(value: f32) -> f32 {
    if value < -1.0 { -1.0 } else { 1.0f32.min(value) }
}

#[inline(always)]
fn half_shift(noise: f32) -> f32 {
    (noise + 1.0) * 0.5
}

#[inline(always)]
fn fade_out(t: f32, start: f32) -> f32 {
    if t < 0.0 {
        start
    } else if t > 1.0 {
        0.0
    } else {
        t * -start + start
    }
}

impl CaveNoises {
    fn new(factory: &PositionalFactory) -> Self {
        let make = |name: &str, first_octave: i32, amplitudes: &[f64]| NormalNoise::new(&mut factory.hash_of(&format!("minecraft:{name}")), first_octave, amplitudes);
        Self {
            pillar: make("pillar", -7, &[1.0, 1.0]),
            pillar_rareness: make("pillar_rareness", -8, &[1.0]),
            pillar_thickness: make("pillar_thickness", -8, &[1.0]),
            spaghetti_2d: make("spaghetti_2d", -7, &[1.0]),
            spaghetti_2d_elevation: make("spaghetti_2d_elevation", -8, &[1.0]),
            spaghetti_2d_modulator: make("spaghetti_2d_modulator", -11, &[1.0]),
            spaghetti_2d_thickness: make("spaghetti_2d_thickness", -11, &[1.0]),
            spaghetti_3d_1: make("spaghetti_3d_1", -7, &[1.0]),
            spaghetti_3d_2: make("spaghetti_3d_2", -7, &[1.0]),
            spaghetti_3d_rarity: make("spaghetti_3d_rarity", -11, &[1.0]),
            spaghetti_3d_thickness: make("spaghetti_3d_thickness", -8, &[1.0]),
            spaghetti_roughness: make("spaghetti_roughness", -5, &[1.0]),
            spaghetti_roughness_modulator: make("spaghetti_roughness_modulator", -8, &[1.0]),
            cave_entrance: make("cave_entrance", -7, &[0.4, 0.5, 1.0]),
            cave_layer: make("cave_layer", -8, &[1.0]),
            cave_cheese: make("cave_cheese", -8, &[0.5, 1.0, 2.0, 1.0, 2.0, 1.0, 0.0, 2.0, 0.0]),
        }
    }

    pub fn density(&self, x: f32, y: f32, z: f32, density: f32) -> f32 {
        if density < CAVE_SKIP_BELOW {
            return density;
        }
        let roughness = self.roughness(x, y, z);
        let tunnels = (self.spaghetti_3d(x, y, z) + roughness).min(self.entrance(x, y, z));
        if density < CAVE_CHEESE_BELOW {
            return (tunnels * CAVE_BOOST * TUNNEL_BOOST).min(density);
        }
        let cheese = self.cheese(x, y, z, density);
        let caves = cheese.min(tunnels.min(roughness + self.spaghetti_2d(x, y, z)));
        caves.max(self.pillar(x, y, z)) * CAVE_BOOST
    }

    #[inline(always)]
    fn roughness(&self, x: f32, y: f32, z: f32) -> f32 {
        let modulator = half_shift(self.spaghetti_roughness_modulator.value(x, y, z)) * ROUGHNESS_MODULATOR_SCALE + 0.0;
        (ROUGHNESS_THRESHOLD - self.spaghetti_roughness.value(x, y, z).abs()) * modulator
    }

    #[inline(always)]
    fn spaghetti_3d(&self, x: f32, y: f32, z: f32) -> f32 {
        let rarity = self.spaghetti_3d_rarity.value(x + x, y, z + z);
        let scale = if rarity < -0.5 {
            0.75
        } else if rarity < 0.0 {
            1.0
        } else if rarity < 0.5 {
            1.5
        } else {
            2.0
        };
        let thickness = half_shift(self.spaghetti_3d_thickness.value(x, y, z)) * THREE_D_THICKNESS_RANGE + THREE_D_THICKNESS_MIN;
        let inverse = 1.0 / scale;
        let first = self.spaghetti_3d_1.value(x * inverse, y * inverse, z * inverse);
        let second = self.spaghetti_3d_2.value(x * inverse, y * inverse, z * inverse);
        clamp_unit(((first * scale).abs() - thickness).max((second * scale).abs() - thickness))
    }

    #[inline(always)]
    fn entrance(&self, x: f32, y: f32, z: f32) -> f32 {
        let noise = self.cave_entrance.value(x * ENTRANCE_HORIZONTAL_SCALE, y * ENTRANCE_VERTICAL_SCALE, z * ENTRANCE_HORIZONTAL_SCALE) + ENTRANCE_BIAS;
        let height = (ENTRANCE_FADE_START + y) / ENTRANCE_FADE_SPAN;
        noise + fade_out(height, ENTRANCE_FADE)
    }

    #[inline(always)]
    fn cheese(&self, x: f32, y: f32, z: f32, density: f32) -> f32 {
        let noise = clamp_unit(self.cave_cheese.value(x, CHEESE_Y_SCALE * y, z) + CHEESE_BIAS);
        let fade = (density + -CAVE_CHEESE_BELOW) * CHEESE_FADE_SCALE;
        let layer = self.cave_layer.value(x, LAYER_Y_SCALE * y, z);
        noise + fade_out(fade, CHEESE_FADE) + layer * layer * LAYER_WEIGHT
    }

    #[inline(always)]
    fn spaghetti_2d(&self, x: f32, y: f32, z: f32) -> f32 {
        let rarity = self.spaghetti_2d_modulator.value(x + x, y, z + z);
        let scale = if rarity < -0.75 {
            0.5
        } else if rarity < -0.5 {
            0.75
        } else if rarity < 0.5 {
            1.0
        } else if rarity < 0.75 {
            2.0
        } else {
            3.0
        };
        let thickness_noise = self.spaghetti_2d_thickness.value(x + x, y, z + z);
        let inverse = 1.0 / scale;
        let sheet = self.spaghetti_2d.value(x * inverse, y * inverse, z * inverse);
        let elevation_noise = self.spaghetti_2d_elevation.value(x, 0.0, z);
        let elevation = if (MIN_CELL_Y + -ELEVATION_MAX).abs() > ELEVATION_EPSILON {
            MIN_CELL_Y + (ELEVATION_MAX - MIN_CELL_Y) * half_shift(elevation_noise)
        } else {
            MIN_CELL_Y
        };
        let thickness = half_shift(thickness_noise) * TWO_D_THICKNESS_RANGE + TWO_D_THICKNESS_MIN;
        let ridge = (scale * sheet).abs() + RIDGE_THICKNESS_FACTOR * thickness;
        let offset = (ELEVATION_SLOPE * y + elevation).abs() - thickness;
        clamp_unit((offset * offset * offset).max(ridge))
    }

    #[inline(always)]
    fn pillar(&self, x: f32, y: f32, z: f32) -> f32 {
        let rareness = half_shift(self.pillar_rareness.value(x, y, z)) * PILLAR_RARENESS_SCALE + 0.0;
        let thickness = (half_shift(self.pillar_thickness.value(x, y, z)) * PILLAR_THICKNESS_SCALE + 0.0).powf(3.0);
        let value = (self.pillar.value(x * PILLAR_HORIZONTAL_SCALE, y * PILLAR_VERTICAL_SCALE, z * PILLAR_HORIZONTAL_SCALE) * 2.0 - rareness) * thickness;
        if value > PILLAR_THRESHOLD { value } else { f32::MIN }
    }
}

pub struct BedrockTerrain {
    temperature: NormalNoise,
    vegetation: NormalNoise,
    continentalness: NormalNoise,
    erosion: NormalNoise,
    ridge: NormalNoise,
    offset: NormalNoise,
    jagged: NormalNoise,
    caves: CaveNoises,
    blended: LegacyBlendedNoise,
    offset_spline: Spline,
    factor_spline: Spline,
    jaggedness_spline: Spline,
    frequency: ClimateScale,
    global_offset: f32,
    factor_scaler: f32,
    sea_level: i32,
    aquifers: bool,
    ore_veins: bool,
    definition: DimensionDefinition,
}

pub struct ClimateColumn {
    temperature: f32,
    humidity: f32,
    continents: f32,
    erosion: f32,
    weirdness: f32,
    offset: f32,
}

impl ClimateColumn {
    pub fn point(&self, y: i32) -> [i64; 6] {
        let depth = y as f32 * DEPTH_PER_BLOCK + 1.0 + self.offset;
        [self.temperature, self.humidity, self.continents, self.erosion, depth, self.weirdness].map(|value| (value as f64 * QUANTISE_SCALE) as i64)
    }
}

struct ColumnShape {
    offset: f32,
    factor: f32,
    jaggedness: f32,
}

impl BedrockTerrain {
    pub fn new(factory: &PositionalFactory, settings: &OverworldSettings, definition: DimensionDefinition) -> Self {
        let shape = &settings.terrain;
        let scale = &settings.climate;
        let make = |name: &str, first_octave: i32, amplitudes: &[f64]| NormalNoise::new(&mut factory.hash_of(&format!("minecraft:{name}")), first_octave, amplitudes);
        Self {
            temperature: make("temperature", -10, &[1.5, 0.0, 1.0, 0.0, 0.0, 0.0]),
            vegetation: make("vegetation", -8, &[1.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
            continentalness: make("continentalness", -9, &[1.0, 1.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0]),
            erosion: make("erosion", -9, &[1.0, 1.0, 0.0, 1.0, 1.0]),
            ridge: make("ridge", -7, &[1.0, 2.0, 1.0, 0.0, 0.0, 0.0]),
            offset: make("offset", -3, &[1.0, 1.0, 1.0, 0.0]),
            jagged: make("jagged", -16, &[1.0; 16]),
            caves: CaveNoises::new(factory),
            blended: LegacyBlendedNoise::new(factory),
            offset_spline: overworld_offset(shape.offset),
            factor_spline: overworld_factor(shape.factor),
            jaggedness_spline: overworld_jaggedness(shape.jaggedness),
            frequency: ClimateScale {
                temperature: 1.0 / scale.temperature,
                humidity: 1.0 / scale.humidity,
                continentalness: 1.0 / scale.continentalness,
                erosion: 1.0 / scale.erosion,
                weirdness: 1.0 / scale.weirdness,
            },
            global_offset: shape.global_offset,
            factor_scaler: shape.factor_scaler,
            sea_level: shape.sea_level,
            aquifers: shape.aquifers,
            ore_veins: shape.ore_veins,
            definition,
        }
    }

    pub fn definition(&self) -> DimensionDefinition {
        self.definition
    }

    pub fn min_y(&self) -> i32 {
        self.definition.min_y
    }

    pub fn cells_y(&self) -> i32 {
        self.definition.height / CELL_HEIGHT
    }

    pub fn noise_levels(&self) -> usize {
        (self.cells_y() - SLIDE_TOP_CELLS + 1) as usize
    }

    pub fn corner_levels(&self) -> usize {
        (self.cells_y() + 1) as usize
    }

    pub fn sea_level(&self) -> i32 {
        self.sea_level
    }

    pub fn aquifers_enabled(&self) -> bool {
        self.aquifers
    }

    pub fn ore_veins_enabled(&self) -> bool {
        self.ore_veins
    }

    pub fn global_offset(&self) -> f32 {
        self.global_offset
    }

    fn shifted(&self, quart_x: f32, quart_z: f32) -> (f32, f32) {
        let x = quart_x + self.offset.value(quart_x, 0.0, quart_z) * SHIFT_SCALE;
        let z = quart_z + self.offset.value(quart_z, quart_x, 0.0) * SHIFT_SCALE;
        (x, z)
    }

    fn spline_input(&self, x: f32, z: f32) -> SplineInput {
        let frequency = &self.frequency;
        let continents = self.continentalness.value(x * frequency.continentalness, 0.0, z * frequency.continentalness);
        let erosion = self.erosion.value(x * frequency.erosion, 0.0, z * frequency.erosion);
        let weirdness = self.ridge.value(x * frequency.weirdness, 0.0, z * frequency.weirdness);
        let ridges = ((weirdness.abs() + PEAK_OFFSET).abs() + VALLEY_OFFSET) * PEAK_SCALE;
        SplineInput {
            continents,
            erosion,
            weirdness,
            ridges,
        }
    }

    fn offset_of(&self, input: &SplineInput) -> f32 {
        self.offset_spline.sample(input) + self.global_offset
    }

    fn factor_of(&self, input: &SplineInput) -> f32 {
        self.factor_spline.sample(input) * self.factor_scaler
    }

    pub fn erosion_and_offset(&self, quart_x: i32, quart_z: i32) -> (f32, f32) {
        let (shifted_x, shifted_z) = self.shifted(quart_x as f32, quart_z as f32);
        let input = self.spline_input(shifted_x, shifted_z);
        (input.erosion, self.offset_spline.sample(&input))
    }

    pub fn climate_column(&self, x: i32, z: i32) -> ClimateColumn {
        let (shifted_x, shifted_z) = self.shifted((x >> 2) as f32, (z >> 2) as f32);
        let input = self.spline_input(shifted_x, shifted_z);
        let frequency = &self.frequency;
        ClimateColumn {
            temperature: self.temperature.value(shifted_x * frequency.temperature, 0.0, shifted_z * frequency.temperature),
            humidity: self.vegetation.value(shifted_x * frequency.humidity, 0.0, shifted_z * frequency.humidity),
            continents: input.continents,
            erosion: input.erosion,
            weirdness: input.weirdness,
            offset: self.offset_of(&input),
        }
    }

    pub fn preliminary_surface(&self, quart_x: i32, quart_z: i32) -> i32 {
        let (shifted_x, shifted_z) = self.shifted(quart_x as f32, quart_z as f32);
        let input = self.spline_input(shifted_x, shifted_z);
        let offset = self.offset_of(&input);
        let factor = self.factor_of(&input);
        let mut y = self.definition.top() - SURFACE_TOP_MARGIN;
        for (steps, counter) in (1..=(self.definition.height - SURFACE_TOP_MARGIN) / CELL_HEIGHT + 1).rev().enumerate() {
            let mut density = (y as f32 * DEPTH_PER_BLOCK + 1.0 + offset) * factor;
            density *= if density > 0.0 { POSITIVE_DENSITY_SCALE } else { 1.0 };
            density += SURFACE_BOTTOM;
            let top = (steps + 1) as f32 * 0.5;
            if top < 0.0 {
                density = SLIDE_TOP_FLOOR;
            } else if top <= 1.0 {
                density = (density + SLIDE_TOP_OFFSET) * top + SLIDE_TOP_FLOOR;
            }
            let bottom = ((counter - 1) & 0xff) as f32 / 3.0;
            if bottom <= 1.0 {
                density = (density + SLIDE_BOTTOM_OFFSET) * bottom + SLIDE_BOTTOM_TARGET;
            }
            if density > SURFACE_THRESHOLD {
                return y;
            }
            y -= CELL_HEIGHT;
        }
        SURFACE_NOT_FOUND
    }

    fn column_shape(&self, quart_x: i32, quart_z: i32) -> ColumnShape {
        let (shifted_x, shifted_z) = self.shifted(quart_x as f32, quart_z as f32);
        let input = self.spline_input(shifted_x, shifted_z);
        let jaggedness_value = self.jaggedness_spline.sample(&input);
        let mut jaggedness = 0.0;
        if jaggedness_value > 0.0 {
            let noise = self.jagged.value(quart_x as f32 * 4.0 * JAGGED_INPUT_SCALE, 0.0, quart_z as f32 * 4.0 * JAGGED_INPUT_SCALE);
            jaggedness = if noise > 0.0 {
                jaggedness_value * noise
            } else {
                (jaggedness_value * JAGGED_NEGATIVE_FACTOR) * noise
            };
        }
        ColumnShape {
            offset: self.offset_of(&input),
            factor: self.factor_of(&input),
            jaggedness,
        }
    }

    pub fn density_corners(&self, chunk_x: i32, chunk_z: i32, buffers: &mut TerrainBuffers, out: &mut Vec<f32>) {
        let (min_y, noise_levels) = (self.definition.min_y, self.noise_levels());
        let position = [(chunk_x * CELL_WIDTH) as f32, min_y.div_euclid(CELL_HEIGHT) as f32, (chunk_z * CELL_WIDTH) as f32];
        self.blended.fill(buffers, position, noise_levels);
        out.clear();
        out.reserve(GRID_COLUMNS * GRID_COLUMNS * noise_levels);
        let mut index = 0;
        for cx in 0..GRID_COLUMNS as i32 {
            for cz in 0..GRID_COLUMNS as i32 {
                let quart_x = chunk_x * CELL_WIDTH + cx;
                let quart_z = chunk_z * CELL_WIDTH + cz;
                let shape = self.column_shape(quart_x, quart_z);
                for cy in 0..noise_levels as i32 {
                    let y = min_y + CELL_HEIGHT * cy;
                    let minimum = buffers.min_limit[index] * LIMIT_FACTOR;
                    let blend = (buffers.main[index] / MAIN_DIVISOR + 1.0) * 0.5;
                    let blended = if 0.0 > blend {
                        minimum
                    } else {
                        let maximum = buffers.max_limit[index] * LIMIT_FACTOR;
                        if blend > 1.0 { maximum } else { minimum + blend * (maximum - minimum) }
                    };
                    let mut base = (y as f32 * DEPTH_PER_BLOCK + 1.0 + shape.offset + shape.jaggedness) * shape.factor;
                    base *= if base > 0.0 { POSITIVE_DENSITY_SCALE } else { 1.0 };
                    let mut density = base + blended;
                    density = self.caves.density((quart_x * 4) as f32, y as f32, (quart_z * 4) as f32, density);
                    let top = (self.cells_y() - SLIDE_TOP_CELLS - cy) as f32 * 0.5;
                    density = if 0.0 > top {
                        SLIDE_TOP_FLOOR
                    } else if top > 1.0 {
                        density
                    } else {
                        (density + SLIDE_TOP_OFFSET) * top + SLIDE_TOP_FLOOR
                    };
                    let bottom = cy as f32 / 3.0;
                    density = if bottom > 1.0 { density } else { (density + SLIDE_BOTTOM_OFFSET) * bottom + SLIDE_BOTTOM_TARGET };
                    out.push(density);
                    index += 1;
                }
            }
        }
    }
}
