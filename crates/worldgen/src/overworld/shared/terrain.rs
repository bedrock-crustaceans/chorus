use super::noise::{BlendedNoise, MAX_COLUMN, NoiseStack, NormalNoise, lerp, lerp3};
use super::random::PositionalFactory;
use super::spline::{Spline, SplineInput, overworld_factor, overworld_jaggedness, overworld_offset};

pub const MIN_Y: i32 = -64;
pub const HEIGHT: i32 = 384;
pub const CELL_WIDTH: i32 = 4;
pub const CELL_HEIGHT: i32 = 8;
const GLOBAL_OFFSET: f32 = -0.50375;
const SURFACE_DENSITY_THRESHOLD: f32 = 1.5625;
const CHEESE_NOISE_TARGET: f32 = -0.703125;
const NOISE_ZERO: f32 = 0.390625;
const ORE_VEIN_MIN_Y: i32 = -64;
const ORE_VEIN_MAX_Y: i32 = 56;

fn gradient(value: i32, from: i32, to: i32, from_value: f32, to_value: f32) -> f32 {
    let factor = (to_value - from_value) / (to - from) as f32;
    from_value + (value.clamp(from.min(to), from.max(to)) - from) as f32 * factor
}

fn lerp_choice(alpha: f32, first: f32, second: impl FnOnce() -> f32) -> f32 {
    if alpha == 0.0 {
        first
    } else if alpha == 1.0 {
        second()
    } else {
        lerp(alpha, first, second())
    }
}

fn mul(left: f32, right: impl FnOnce() -> f32) -> f32 {
    if left == 0.0 { 0.0 } else { left * right() }
}

fn half_negative(v: f32) -> f32 {
    if v > 0.0 { v } else { v * 0.5 }
}

fn quarter_negative(v: f32) -> f32 {
    if v > 0.0 { v } else { v * 0.25 }
}

fn remap(v: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    let factor = (to_max - to_min) / (from_max - from_min);
    let offset = to_min - from_min * factor;
    if offset == 0.0 { v * factor } else { v * factor + offset }
}

fn squeeze(v: f32) -> f32 {
    let c = v.clamp(-1.0, 1.0);
    c / 2.0 - c * c * c / 24.0
}

fn slide(y: i32, value: impl FnOnce() -> f32) -> f32 {
    let top = gradient(y, MIN_Y + HEIGHT - 80, MIN_Y + HEIGHT - 64, 1.0, 0.0);
    let bottom = gradient(y, MIN_Y, MIN_Y + 24, 0.0, 1.0);
    lerp_choice(bottom, 0.1171875, || lerp_choice(top, -0.078125, value))
}

fn slide_uses_value(y: i32) -> bool {
    gradient(y, MIN_Y, MIN_Y + 24, 0.0, 1.0) != 0.0 && gradient(y, MIN_Y + HEIGHT - 80, MIN_Y + HEIGHT - 64, 1.0, 0.0) != 0.0
}

const NOODLE_RANGE: (i32, i32) = (-60, 320);
const VEIN_RANGE: (i32, i32) = (ORE_VEIN_MIN_Y, ORE_VEIN_MAX_Y);

fn in_range(y: i32, (min, max): (i32, i32)) -> bool {
    y >= min && y < max + 1
}

fn spaghetti_roughness(modulator: f32, roughness: f32) -> f32 {
    mul(remap(modulator, -1.0, 1.0, 0.0, -0.1), || roughness.abs() + -0.4)
}

fn spaghetti_3d_rarity(value: f32) -> f32 {
    if value < -0.5 {
        0.75
    } else if value < 0.0 {
        1.0
    } else if value < 0.5 {
        1.5
    } else {
        2.0
    }
}

fn spaghetti_2d_rarity(value: f32) -> f32 {
    if value < -0.75 {
        0.5
    } else if value < -0.5 {
        0.75
    } else if value < 0.5 {
        1.0
    } else if value < 0.75 {
        2.0
    } else {
        3.0
    }
}

fn spaghetti_3d(rarity: f32, first: f32, second: f32, thickness: f32) -> f32 {
    let thickness = remap(thickness, -1.0, 1.0, -0.065, -0.088);
    ((first * rarity).abs().max((second * rarity).abs()) + thickness).clamp(-1.0, 1.0)
}

fn entrances(y: i32, cave_entrance: f32, roughness: f32, spaghetti: f32) -> f32 {
    let big = cave_entrance + 0.37 + gradient(y, -10, 30, 0.3, 0.0);
    big.min(roughness + spaghetti)
}

fn spaghetti_2d(y: i32, rarity: f32, cave: f32, elevation: f32, thickness: f32) -> f32 {
    let cave = (cave * rarity).abs();
    let elevation = remap(elevation, -1.0, 1.0, -8.0, 8.0);
    let thickness = remap(thickness, -1.0, 1.0, -0.6, -1.3);
    let sloped = (elevation + gradient(y, -64, 320, 8.0, -40.0)).abs();
    let ridged = sloped + thickness;
    let layer_ridged = ridged * ridged * ridged;
    (cave + thickness * 0.083).max(layer_ridged).clamp(-1.0, 1.0)
}

fn pillars(pillar: f32, rareness: f32, thickness: f32) -> f32 {
    let rareness = remap(rareness, -1.0, 1.0, 0.0, -2.0);
    let thickness = remap(thickness, -1.0, 1.0, 0.0, 1.1);
    mul(pillar * 2.0 + rareness, || thickness * thickness * thickness)
}

fn underground(sloped_cheese: f32, entrances: f32, layer: f32, cheese: f32, spaghetti: f32, pillars: f32) -> f32 {
    let layerized = layer * layer * 4.0;
    let solidified = (cheese + 0.27).clamp(-1.0, 1.0) + (sloped_cheese * -0.64 + 1.5).clamp(0.0, 0.5);
    let subtractions = (layerized + solidified).min(entrances).min(spaghetti);
    subtractions.max(if pillars < 0.03 { -1000000.0 } else { pillars })
}

struct ColumnSampler<'a> {
    x: i32,
    z: i32,
    ys: &'a [i32],
}

impl ColumnSampler<'_> {
    fn sample(&self, noise: &Noise, indices: &[usize], xz_scale: f64, y_scale: f64) -> [f32; MAX_COLUMN] {
        let mut ys = [0.0f64; MAX_COLUMN];
        for (scaled, &y) in ys.iter_mut().zip(self.ys) {
            *scaled = y as f64 * y_scale;
        }
        let mut out = [0.0; MAX_COLUMN];
        noise.0.column_subset(self.x as f64 * xz_scale, self.z as f64 * xz_scale, &ys, indices, &mut out);
        out
    }

    fn sample_by_rarity(&self, noise: &Noise, indices: &[usize], rarity: &[f32]) -> [f32; MAX_COLUMN] {
        let mut groups: Vec<(f32, Vec<usize>)> = Vec::new();
        for &i in indices {
            match groups.iter_mut().find(|(value, _)| *value == rarity[i]) {
                Some((_, group)) => group.push(i),
                None => groups.push((rarity[i], vec![i])),
            }
        }
        let mut out = [0.0; MAX_COLUMN];
        for (value, group) in groups {
            let scale = 1.0 / value as f64;
            let part = self.sample(noise, &group, scale, scale);
            for i in group {
                out[i] = part[i];
            }
        }
        out
    }
}

struct Noise(NoiseStack);

impl Noise {
    fn new(random: &PositionalFactory, name: &str, first_octave: i32, amplitudes: &[f64]) -> Self {
        Self(NormalNoise::parity(first_octave, amplitudes).create(&mut random.hash_of(&format!("minecraft:{name}"))))
    }

    fn at(&self, x: f64, y: f64, z: f64) -> f32 {
        self.0.get(x, y, z)
    }

    fn scaled(&self, x: i32, y: i32, z: i32, xz_scale: f64, y_scale: f64) -> f32 {
        self.0.get(x as f64 * xz_scale, y as f64 * y_scale, z as f64 * xz_scale)
    }
}

pub struct SurfaceNoises {
    pub surface: NoiseStack,
    pub surface_secondary: NoiseStack,
    pub clay_bands_offset: NoiseStack,
    pub badlands_pillar: NoiseStack,
    pub badlands_pillar_roof: NoiseStack,
    pub badlands_surface: NoiseStack,
    pub iceberg_pillar: NoiseStack,
    pub iceberg_pillar_roof: NoiseStack,
    pub iceberg_surface: NoiseStack,
    pub swamp: NoiseStack,
    pub calcite: NoiseStack,
    pub gravel: NoiseStack,
    pub powder_snow: NoiseStack,
    pub packed_ice: NoiseStack,
    pub ice: NoiseStack,
    pub sulfur_cave_gradient: NoiseStack,
    pub surface_patch_small: NoiseStack,
}

impl SurfaceNoises {
    fn new(random: &PositionalFactory) -> Self {
        let n = |name: &str, first: i32, amplitudes: &[f64]| Noise::new(random, name, first, amplitudes).0;
        Self {
            surface: n("surface", -6, &[1.0, 1.0, 1.0]),
            surface_secondary: n("surface_secondary", -6, &[1.0, 1.0, 0.0, 1.0]),
            clay_bands_offset: n("clay_bands_offset", -8, &[1.0]),
            badlands_pillar: n("badlands_pillar", -2, &[1.0, 1.0, 1.0, 1.0]),
            badlands_pillar_roof: n("badlands_pillar_roof", -8, &[1.0]),
            badlands_surface: n("badlands_surface", -6, &[1.0, 1.0, 1.0]),
            iceberg_pillar: n("iceberg_pillar", -6, &[1.0, 1.0, 1.0, 1.0]),
            iceberg_pillar_roof: n("iceberg_pillar_roof", -3, &[1.0]),
            iceberg_surface: n("iceberg_surface", -6, &[1.0, 1.0, 1.0]),
            swamp: n("surface_swamp", -2, &[1.0]),
            calcite: n("calcite", -9, &[1.0, 1.0, 1.0, 1.0]),
            gravel: n("gravel", -8, &[1.0, 1.0, 1.0, 1.0]),
            powder_snow: n("powder_snow", -6, &[1.0, 1.0, 1.0, 1.0]),
            packed_ice: n("packed_ice", -7, &[1.0, 1.0, 1.0, 1.0]),
            ice: n("ice", -4, &[1.0, 1.0, 1.0, 1.0]),
            sulfur_cave_gradient: n("sulfur_cave_gradient", -5, &[1.0, 0.0, 1.0]),
            surface_patch_small: n("small_patch", -3, &[3.0]),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Column {
    pub continents: f32,
    pub erosion: f32,
    pub ridges: f32,
    pub offset: f32,
    pub factor: f32,
    pub jaggedness: f32,
}

pub struct Climate {
    pub temperature: f32,
    pub vegetation: f32,
    pub continents: f32,
    pub erosion: f32,
    pub depth: f32,
    pub ridges: f32,
}

pub struct SurfaceClimate {
    temperature: f32,
    vegetation: f32,
    continents: f32,
    erosion: f32,
    ridges: f32,
    offset: f32,
}

impl SurfaceClimate {
    pub fn at(&self, y: i32) -> Climate {
        Climate {
            temperature: self.temperature,
            vegetation: self.vegetation,
            continents: self.continents,
            erosion: self.erosion,
            depth: depth(self.offset, y),
            ridges: self.ridges,
        }
    }
}

fn depth(offset: f32, y: i32) -> f32 {
    gradient(y, -64, 320, 1.5, -1.5) + offset
}

pub struct Terrain {
    shift: Noise,
    temperature: Noise,
    vegetation: Noise,
    continentalness: Noise,
    erosion: Noise,
    ridge: Noise,
    jagged: Noise,
    base_3d: BlendedNoise,
    offset_spline: Spline,
    factor_spline: Spline,
    jaggedness_spline: Spline,

    cave_entrance: Noise,
    cave_layer: Noise,
    cave_cheese: Noise,
    spaghetti_3d_rarity: Noise,
    spaghetti_3d_thickness: Noise,
    spaghetti_3d_1: Noise,
    spaghetti_3d_2: Noise,
    spaghetti_roughness: Noise,
    spaghetti_roughness_modulator: Noise,
    spaghetti_2d: Noise,
    spaghetti_2d_modulator: Noise,
    spaghetti_2d_elevation: Noise,
    spaghetti_2d_thickness: Noise,
    pillar: Noise,
    pillar_rareness: Noise,
    pillar_thickness: Noise,
    noodle: Noise,
    noodle_thickness: Noise,
    noodle_ridge_a: Noise,
    noodle_ridge_b: Noise,

    ore_veininess: Noise,
    ore_vein_a: Noise,
    ore_vein_b: Noise,
    ore_gap: Noise,

    aquifer_barrier: Noise,
    aquifer_floodedness: Noise,
    aquifer_spread: Noise,
    aquifer_lava: Noise,

    pub surface: SurfaceNoises,
}

impl Terrain {
    pub fn new(random: &PositionalFactory) -> Self {
        let n = |name: &str, first: i32, amplitudes: &[f64]| Noise::new(random, name, first, amplitudes);
        Self {
            shift: n("offset", -3, &[1.0, 1.0, 1.0, 0.0]),
            temperature: n("temperature", -10, &[1.5, 0.0, 1.0, 0.0, 0.0, 0.0]),
            vegetation: n("vegetation", -8, &[1.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
            continentalness: n("continentalness", -9, &[1.0, 1.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0]),
            erosion: n("erosion", -9, &[1.0, 1.0, 0.0, 1.0, 1.0]),
            ridge: n("ridge", -7, &[1.0, 2.0, 1.0, 0.0, 0.0, 0.0]),
            jagged: n("jagged", -16, &[1.0; 16]),
            base_3d: BlendedNoise::new(&mut random.hash_of("minecraft:terrain"), 0.25, 0.125, 80.0, 160.0, 8.0),
            offset_spline: overworld_offset(),
            factor_spline: overworld_factor(),
            jaggedness_spline: overworld_jaggedness(),

            cave_entrance: n("cave_entrance", -7, &[0.4, 0.5, 1.0]),
            cave_layer: n("cave_layer", -8, &[1.0]),
            cave_cheese: n("cave_cheese", -8, &[0.5, 1.0, 2.0, 1.0, 2.0, 1.0, 0.0, 2.0, 0.0]),
            spaghetti_3d_rarity: n("spaghetti_3d_rarity", -11, &[1.0]),
            spaghetti_3d_thickness: n("spaghetti_3d_thickness", -8, &[1.0]),
            spaghetti_3d_1: n("spaghetti_3d_1", -7, &[1.0]),
            spaghetti_3d_2: n("spaghetti_3d_2", -7, &[1.0]),
            spaghetti_roughness: n("spaghetti_roughness", -5, &[1.0]),
            spaghetti_roughness_modulator: n("spaghetti_roughness_modulator", -8, &[1.0]),
            spaghetti_2d: n("spaghetti_2d", -7, &[1.0]),
            spaghetti_2d_modulator: n("spaghetti_2d_modulator", -11, &[1.0]),
            spaghetti_2d_elevation: n("spaghetti_2d_elevation", -8, &[1.0]),
            spaghetti_2d_thickness: n("spaghetti_2d_thickness", -11, &[1.0]),
            pillar: n("pillar", -7, &[1.0, 1.0]),
            pillar_rareness: n("pillar_rareness", -8, &[1.0]),
            pillar_thickness: n("pillar_thickness", -8, &[1.0]),
            noodle: n("noodle", -8, &[1.0]),
            noodle_thickness: n("noodle_thickness", -8, &[1.0]),
            noodle_ridge_a: n("noodle_ridge_a", -7, &[1.0]),
            noodle_ridge_b: n("noodle_ridge_b", -7, &[1.0]),

            ore_veininess: n("ore_veininess", -8, &[1.0]),
            ore_vein_a: n("ore_vein_a", -7, &[1.0]),
            ore_vein_b: n("ore_vein_b", -7, &[1.0]),
            ore_gap: n("ore_gap", -5, &[1.0]),

            aquifer_barrier: n("aquifer_barrier", -3, &[1.0]),
            aquifer_floodedness: n("aquifer_fluid_level_floodedness", -7, &[1.0]),
            aquifer_spread: n("aquifer_fluid_level_spread", -5, &[1.0]),
            aquifer_lava: n("aquifer_lava", -1, &[1.0]),

            surface: SurfaceNoises::new(random),
        }
    }

    fn shift_x(&self, x: i32, z: i32) -> f64 {
        (self.shift.at(x as f64 * 0.25, 0.0, z as f64 * 0.25) * 4.0) as f64
    }

    fn shift_z(&self, x: i32, z: i32) -> f64 {
        (self.shift.at(z as f64 * 0.25, x as f64 * 0.25, 0.0) * 4.0) as f64
    }

    fn shifted(&self, noise: &Noise, x: i32, z: i32, shift_x: f64, shift_z: f64) -> f32 {
        noise.at(x as f64 * 0.25 + shift_x, 0.0, z as f64 * 0.25 + shift_z)
    }

    fn spline_input(&self, x: i32, z: i32, sx: f64, sz: f64) -> SplineInput {
        let ridges = self.shifted(&self.ridge, x, z, sx, sz);
        SplineInput {
            continents: self.shifted(&self.continentalness, x, z, sx, sz),
            erosion: self.shifted(&self.erosion, x, z, sx, sz),
            weirdness: ridges,
            ridges: (((ridges.abs() + -0.6666667).abs()) + -0.33333334) * -3.0,
        }
    }

    fn column_shape(&self, x: i32, z: i32) -> (Column, SplineInput) {
        let input = self.spline_input(x, z, self.shift_x(x, z), self.shift_z(x, z));
        let column = Column {
            continents: input.continents,
            erosion: input.erosion,
            ridges: input.weirdness,
            offset: GLOBAL_OFFSET + self.offset_spline.sample(&input),
            factor: self.factor_spline.sample(&input),
            jaggedness: 0.0,
        };
        (column, input)
    }

    pub fn column(&self, x: i32, z: i32) -> Column {
        let (column, input) = self.column_shape(x, z);
        let unscaled_jaggedness = self.jaggedness_spline.sample(&input);
        Column {
            jaggedness: mul(unscaled_jaggedness, || half_negative(self.jagged.scaled(x, 0, z, 1500.0, 0.0))),
            ..column
        }
    }

    pub fn climate(&self, x: i32, y: i32, z: i32) -> Climate {
        self.surface_climate(x, z).at(y)
    }

    pub fn surface_climate(&self, x: i32, z: i32) -> SurfaceClimate {
        let (sx, sz) = (self.shift_x(x, z), self.shift_z(x, z));
        let input = self.spline_input(x, z, sx, sz);
        SurfaceClimate {
            temperature: self.shifted(&self.temperature, x, z, sx, sz),
            vegetation: self.shifted(&self.vegetation, x, z, sx, sz),
            continents: input.continents,
            erosion: input.erosion,
            ridges: input.weirdness,
            offset: GLOBAL_OFFSET + self.offset_spline.sample(&input),
        }
    }

    fn depth(column: &Column, y: i32) -> f32 {
        depth(column.offset, y)
    }

    fn initial_density(column: &Column, depth_with_jaggedness: f32) -> f32 {
        quarter_negative(depth_with_jaggedness * column.factor) * 4.0
    }

    pub fn preliminary_surface_level(&self, x: i32, z: i32) -> f32 {
        Self::surface_level_of(&self.column_shape(x, z).0)
    }

    pub fn quart_column(&self, x: i32, z: i32) -> (SurfaceClimate, f32) {
        let (sx, sz) = (self.shift_x(x, z), self.shift_z(x, z));
        let input = self.spline_input(x, z, sx, sz);
        let column = Column {
            continents: input.continents,
            erosion: input.erosion,
            ridges: input.weirdness,
            offset: GLOBAL_OFFSET + self.offset_spline.sample(&input),
            factor: self.factor_spline.sample(&input),
            jaggedness: 0.0,
        };
        let climate = SurfaceClimate {
            temperature: self.shifted(&self.temperature, x, z, sx, sz),
            vegetation: self.shifted(&self.vegetation, x, z, sx, sz),
            continents: column.continents,
            erosion: column.erosion,
            ridges: column.ridges,
            offset: column.offset,
        };
        (climate, Self::surface_level_of(&column))
    }

    fn surface_level_of(column: &Column) -> f32 {
        let upper = remap(0.2734375 / column.factor - column.offset, 1.5, -1.5, -64.0, 320.0).clamp(-40.0, 320.0);
        let density = |y: i32| slide(y, || (Self::initial_density(column, Self::depth(column, y)) + CHEESE_NOISE_TARGET).clamp(-64.0, 64.0)) + -NOISE_ZERO;
        let top = (upper / 8.0).floor() as i32 * 8;
        if top <= -64 {
            return -64.0;
        }
        let mut probe = top;
        while probe >= -64 {
            if density(probe) > 0.0 {
                return probe as f32;
            }
            probe -= 8;
        }
        -64.0
    }

    pub fn aquifer_barrier(&self, x: i32, y: i32, z: i32) -> f32 {
        self.aquifer_barrier.scaled(x, y, z, 1.0, 0.5)
    }

    pub fn aquifer_floodedness(&self, x: i32, y: i32, z: i32) -> f32 {
        self.aquifer_floodedness.scaled(x, y, z, 1.0, 0.67)
    }

    pub fn aquifer_spread(&self, x: i32, y: i32, z: i32) -> f32 {
        self.aquifer_spread.scaled(x, y, z, 1.0, 0.7142857142857143)
    }

    pub fn aquifer_lava(&self, x: i32, y: i32, z: i32) -> f32 {
        self.aquifer_lava.scaled(x, y, z, 1.0, 1.0)
    }

    pub fn aquifer_exclusion(&self, x: i32, y: i32, z: i32) -> f32 {
        let (column, _) = self.column_shape(x, z);
        (-0.225 - column.erosion).min((Self::depth(&column, y) - 0.9).max(0.0))
    }

    pub fn ore_gap(&self, x: i32, y: i32, z: i32) -> f32 {
        -0.3 - self.ore_gap.scaled(x, y, z, 1.0, 1.0)
    }

    fn corner_column(&self, x: i32, z: i32, out: &mut [Corner]) {
        let column = self.column(x, z);
        let ys: Vec<i32> = (0..out.len()).map(|cy| MIN_Y + cy as i32 * CELL_HEIGHT).collect();
        let sampler = ColumnSampler { x, z, ys: &ys };
        let select = |keep: &dyn Fn(i32) -> bool| -> Vec<usize> { (0..ys.len()).filter(|&i| keep(ys[i])).collect() };

        let live = select(&slide_uses_value);
        let mut base = vec![0.0; ys.len()];
        self.base_3d.column(x, z, &ys, &live, &mut base);
        let sloped: Vec<f32> = ys
            .iter()
            .zip(&base)
            .map(|(&y, &base)| Self::initial_density(&column, Self::depth(&column, y) + column.jaggedness) + base)
            .collect();

        let cave_entrance = sampler.sample(&self.cave_entrance, &live, 0.75, 0.5);
        let roughness_modulator = sampler.sample(&self.spaghetti_roughness_modulator, &live, 1.0, 1.0);
        let roughness_noise = sampler.sample(&self.spaghetti_roughness, &live, 1.0, 1.0);
        let rarity_3d: Vec<f32> = sampler.sample(&self.spaghetti_3d_rarity, &live, 2.0, 1.0).into_iter().map(spaghetti_3d_rarity).collect();
        let first_3d = sampler.sample_by_rarity(&self.spaghetti_3d_1, &live, &rarity_3d);
        let second_3d = sampler.sample_by_rarity(&self.spaghetti_3d_2, &live, &rarity_3d);
        let thickness_3d = sampler.sample(&self.spaghetti_3d_thickness, &live, 1.0, 1.0);

        let deep: Vec<usize> = live.iter().copied().filter(|&i| sloped[i] >= SURFACE_DENSITY_THRESHOLD).collect();
        let cave_layer = sampler.sample(&self.cave_layer, &deep, 1.0, 8.0);
        let cave_cheese = sampler.sample(&self.cave_cheese, &deep, 1.0, 0.6666666666666666);
        let rarity_2d: Vec<f32> = sampler.sample(&self.spaghetti_2d_modulator, &deep, 2.0, 1.0).into_iter().map(spaghetti_2d_rarity).collect();
        let cave_2d = sampler.sample_by_rarity(&self.spaghetti_2d, &deep, &rarity_2d);
        let elevation_2d = sampler.sample(&self.spaghetti_2d_elevation, &deep, 1.0, 0.0);
        let thickness_2d = sampler.sample(&self.spaghetti_2d_thickness, &deep, 2.0, 1.0);
        let pillar = sampler.sample(&self.pillar, &deep, 25.0, 0.3);
        let pillar_rareness = sampler.sample(&self.pillar_rareness, &deep, 1.0, 1.0);
        let pillar_thickness = sampler.sample(&self.pillar_thickness, &deep, 1.0, 1.0);

        let noodles = select(&|y| in_range(y, NOODLE_RANGE));
        let noodle = sampler.sample(&self.noodle, &noodles, 1.0, 1.0);
        let noodle_thickness = sampler.sample(&self.noodle_thickness, &noodles, 1.0, 1.0);
        let noodle_ridge_a = sampler.sample(&self.noodle_ridge_a, &noodles, 2.6666666666666665, 2.6666666666666665);
        let noodle_ridge_b = sampler.sample(&self.noodle_ridge_b, &noodles, 2.6666666666666665, 2.6666666666666665);

        let veins = select(&|y| in_range(y, VEIN_RANGE));
        let vein_toggle = sampler.sample(&self.ore_veininess, &veins, 1.5, 1.5);
        let vein_a = sampler.sample(&self.ore_vein_a, &veins, 4.0, 4.0);
        let vein_b = sampler.sample(&self.ore_vein_b, &veins, 4.0, 4.0);

        for (i, corner) in out.iter_mut().enumerate() {
            let y = ys[i];
            let main = slide(y, || {
                let roughness = spaghetti_roughness(roughness_modulator[i], roughness_noise[i]);
                let entrances = entrances(y, cave_entrance[i], roughness, spaghetti_3d(rarity_3d[i], first_3d[i], second_3d[i], thickness_3d[i]));
                if sloped[i] < SURFACE_DENSITY_THRESHOLD {
                    sloped[i].min(entrances * 5.0)
                } else {
                    let spaghetti = spaghetti_2d(y, rarity_2d[i], cave_2d[i], elevation_2d[i], thickness_2d[i]);
                    let pillars = pillars(pillar[i], pillar_rareness[i], pillar_thickness[i]);
                    underground(sloped[i], entrances, cave_layer[i], cave_cheese[i], spaghetti + roughness, pillars)
                }
            });
            let noodled = in_range(y, NOODLE_RANGE);
            let veined = in_range(y, VEIN_RANGE);
            *corner = Corner {
                main: main * 0.64,
                noodle: if noodled { noodle[i] } else { -1.0 },
                noodle_thickness: if noodled { remap(noodle_thickness[i], -1.0, 1.0, -0.05, -0.1) } else { 0.0 },
                noodle_ridge_a: if noodled { noodle_ridge_a[i] } else { 0.0 },
                noodle_ridge_b: if noodled { noodle_ridge_b[i] } else { 0.0 },
                vein_toggle: if veined { vein_toggle[i] } else { 0.0 },
                vein_a: if veined { vein_a[i] } else { 1.0 },
                vein_b: if veined { vein_b[i] } else { 1.0 },
            };
        }
    }

    pub fn corner_columns(&self, chunk_x: i32, chunk_z: i32) -> CornerColumns {
        let mut corners = vec![Corner::default(); (CELLS * CELLS * CORNER_LEVELS) as usize];
        for (index, column) in corners.chunks_mut(CORNER_LEVELS as usize).enumerate() {
            let (cx, cz) = (index as i32 % CELLS, index as i32 / CELLS);
            self.corner_column((chunk_x << 4) + cx * CELL_WIDTH, (chunk_z << 4) + cz * CELL_WIDTH, column);
        }
        CornerColumns(corners)
    }

    pub fn noise_chunk(&self, chunk_x: i32, chunk_z: i32) -> NoiseChunk {
        let mut corners = vec![Corner::default(); (CORNER_SPAN * CORNER_SPAN * CORNER_LEVELS) as usize];
        for (index, column) in corners.chunks_mut(CORNER_LEVELS as usize).enumerate() {
            let (cx, cz) = (index as i32 % CORNER_SPAN, index as i32 / CORNER_SPAN);
            self.corner_column((chunk_x << 4) + cx * CELL_WIDTH, (chunk_z << 4) + cz * CELL_WIDTH, column);
        }
        NoiseChunk { corners, size_y: CORNER_LEVELS }
    }
}

const CORNER_SPAN: i32 = CELLS + 1;

const CELLS: i32 = 16 / CELL_WIDTH;
const CORNER_LEVELS: i32 = HEIGHT / CELL_HEIGHT + 1;

pub struct CornerColumns(Vec<Corner>);

impl CornerColumns {
    fn column(&self, cx: i32, cz: i32) -> &[Corner] {
        let start = ((cz * CELLS + cx) * CORNER_LEVELS) as usize;
        &self.0[start..start + CORNER_LEVELS as usize]
    }
}

#[derive(Clone, Copy, Default)]
pub struct Corner {
    main: f32,
    noodle: f32,
    noodle_thickness: f32,
    noodle_ridge_a: f32,
    noodle_ridge_b: f32,
    vein_toggle: f32,
    vein_a: f32,
    vein_b: f32,
}

pub struct Vein<'a> {
    toggle: f32,
    point: CellPoint<'a>,
}

#[derive(Clone)]
pub struct NoiseChunk {
    corners: Vec<Corner>,
    size_y: i32,
}

struct CellValues {
    main: [f32; 8],
    noodle: [f32; 8],
    noodle_thickness: [f32; 8],
    noodle_ridge_a: [f32; 8],
    noodle_ridge_b: [f32; 8],
}

fn lerp_cell(a: [f32; 3], v: &[f32; 8]) -> f32 {
    lerp3(a[0], a[1], a[2], v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7])
}

impl CellValues {
    fn new(corners: &[&Corner; 8]) -> Self {
        Self {
            main: corners.map(|corner| corner.main),
            noodle: corners.map(|corner| corner.noodle),
            noodle_thickness: corners.map(|corner| corner.noodle_thickness),
            noodle_ridge_a: corners.map(|corner| corner.noodle_ridge_a),
            noodle_ridge_b: corners.map(|corner| corner.noodle_ridge_b),
        }
    }

    fn density(&self, alpha: [f32; 3]) -> f32 {
        let main = squeeze(lerp_cell(alpha, &self.main));
        if lerp_cell(alpha, &self.noodle) < 0.0 {
            return main.min(64.0);
        }
        let thickness = lerp_cell(alpha, &self.noodle_thickness);
        let ridge_a = lerp_cell(alpha, &self.noodle_ridge_a).abs();
        let ridge_b = lerp_cell(alpha, &self.noodle_ridge_b).abs();
        main.min(thickness + ridge_a.max(ridge_b) * 1.5)
    }
}

pub struct Densities(Vec<f32>);

impl Densities {
    pub fn get(&self, lx: i32, y: i32, lz: i32) -> f32 {
        self.0[((lz * 16 + lx) * HEIGHT + y - MIN_Y) as usize]
    }
}

struct CellPoint<'a> {
    alpha: [f32; 3],
    corners: [&'a Corner; 8],
}

impl CellPoint<'_> {
    fn lerp(&self, field: impl Fn(&Corner) -> f32) -> f32 {
        lerp_cell(self.alpha, &self.corners.map(field))
    }
}

impl NoiseChunk {
    pub fn assemble([own, east, south, south_east]: [&CornerColumns; 4]) -> Self {
        let mut corners = Vec::with_capacity((CORNER_SPAN * CORNER_SPAN * CORNER_LEVELS) as usize);
        for cz in 0..CORNER_SPAN {
            for cx in 0..CORNER_SPAN {
                let column = match (cx == CELLS, cz == CELLS) {
                    (false, false) => own.column(cx, cz),
                    (true, false) => east.column(0, cz),
                    (false, true) => south.column(cx, 0),
                    (true, true) => south_east.column(0, 0),
                };
                corners.extend_from_slice(column);
            }
        }
        Self { corners, size_y: CORNER_LEVELS }
    }

    fn corner(&self, cx: i32, cy: i32, cz: i32) -> &Corner {
        &self.corners[((cz * 5 + cx) * self.size_y + cy) as usize]
    }

    fn point(&self, lx: i32, y: i32, lz: i32) -> CellPoint<'_> {
        let ry = y - MIN_Y;
        let (cx, cy, cz) = (lx / CELL_WIDTH, ry / CELL_HEIGHT, lz / CELL_WIDTH);
        let ny = (cy + 1).min(self.size_y - 1);
        CellPoint {
            alpha: [
                (lx % CELL_WIDTH) as f32 / CELL_WIDTH as f32,
                (ry % CELL_HEIGHT) as f32 / CELL_HEIGHT as f32,
                (lz % CELL_WIDTH) as f32 / CELL_WIDTH as f32,
            ],
            corners: [
                self.corner(cx, cy, cz),
                self.corner(cx + 1, cy, cz),
                self.corner(cx, ny, cz),
                self.corner(cx + 1, ny, cz),
                self.corner(cx, cy, cz + 1),
                self.corner(cx + 1, cy, cz + 1),
                self.corner(cx, ny, cz + 1),
                self.corner(cx + 1, ny, cz + 1),
            ],
        }
    }

    pub fn densities(&self) -> Densities {
        let mut values = vec![0.0; (16 * 16 * HEIGHT) as usize];
        for cz in 0..16 / CELL_WIDTH {
            for cx in 0..16 / CELL_WIDTH {
                for cy in 0..HEIGHT / CELL_HEIGHT {
                    let corners = [
                        self.corner(cx, cy, cz),
                        self.corner(cx + 1, cy, cz),
                        self.corner(cx, cy + 1, cz),
                        self.corner(cx + 1, cy + 1, cz),
                        self.corner(cx, cy, cz + 1),
                        self.corner(cx + 1, cy, cz + 1),
                        self.corner(cx, cy + 1, cz + 1),
                        self.corner(cx + 1, cy + 1, cz + 1),
                    ];
                    let cell = CellValues::new(&corners);
                    for iz in 0..CELL_WIDTH {
                        for ix in 0..CELL_WIDTH {
                            let column = ((cz * CELL_WIDTH + iz) * 16 + cx * CELL_WIDTH + ix) * HEIGHT + cy * CELL_HEIGHT;
                            for iy in 0..CELL_HEIGHT {
                                let alpha = [ix as f32 / CELL_WIDTH as f32, iy as f32 / CELL_HEIGHT as f32, iz as f32 / CELL_WIDTH as f32];
                                values[(column + iy) as usize] = cell.density(alpha);
                            }
                        }
                    }
                }
            }
        }
        Densities(values)
    }

    pub fn vein(&self, lx: i32, y: i32, lz: i32) -> Vein<'_> {
        let point = self.point(lx, y, lz);
        Vein {
            toggle: point.lerp(|corner| corner.vein_toggle),
            point,
        }
    }
}

impl Vein<'_> {
    pub fn density(&self, y: i32, min_y: i32, max_y: i32, toggle_positive: bool) -> f32 {
        if !(y >= min_y && y < max_y) || (self.toggle >= -0.4 && self.toggle < 0.4) {
            return -1.0;
        }
        let a = self.point.lerp(|corner| corner.vein_a);
        let b = self.point.lerp(|corner| corner.vein_b);
        if 0.08 - a.abs().max(b.abs()) < 0.0 {
            return -1.0;
        }
        let distance_from_edge = ((max_y - y) as f32).min((y - min_y) as f32);
        let edge_roundoff = remap(distance_from_edge.clamp(0.0, 20.0), 0.0, 20.0, -0.2, 0.0);
        let veininess = if toggle_positive { self.toggle } else { -self.toggle };
        if veininess - 0.4 + edge_roundoff >= 0.0 { 0.7 } else { -1.0 }
    }

    pub fn richness(&self) -> f32 {
        remap(self.toggle.abs().clamp(0.4, 0.6), 0.4, 0.6, 0.1, 0.3)
    }
}
