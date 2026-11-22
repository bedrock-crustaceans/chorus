use super::column::Column;
use super::palette::Palette;
use crate::bedrock::overworld::noise::NormalNoise;
use crate::shared::rng::random::RandomSource;
use crate::shared::rng::xoroshiro::PositionalFactory;

const SURFACE_NAME: &str = "minecraft:iceberg_surface";
const SURFACE_OCTAVE: i32 = -6;
const PILLAR_NAME: &str = "minecraft:iceberg_pillar";
const PILLAR_OCTAVE: i32 = -6;
const ROOF_NAME: &str = "minecraft:iceberg_pillar_roof";
const ROOF_OCTAVE: i32 = -3;
const SURFACE_SCALE: f32 = 8.25;
const PILLAR_INPUT: f32 = 1.28;
const PILLAR_SCALE: f32 = 15.0;
const ROOF_INPUT: f32 = 1.17;
const ROOF_SCALE: f32 = 1.5;
const ROOF_HEIGHT_SCALE: f32 = 40.0;
const MIN_HEIGHT: f32 = 1.8;
const HEIGHT_CURVE: f32 = 1.2;
const ROOF_CEILING_OFFSET: f32 = 14.0;
const WARM_TEMPERATURE: f32 = 0.1;
const WARM_LOWERING: f32 = -2.0;
const SOLID_HEIGHT: f32 = 2.0;
const UNDERWATER_DEPTH: f32 = 7.0;
const SNOW_LIMIT_BASE: i32 = 2;
const SNOW_LIMIT_RANDOM: i32 = 4;
const SNOW_FLOOR_RANDOM: i32 = 10;
const SNOW_FLOOR_ABOVE_SEA: i32 = 18;
const AIR_PLACE_CHANCE: f32 = 0.01;
const WATER_PLACE_CHANCE: f32 = 0.15;

pub struct Iceberg {
    surface: NormalNoise,
    pillar: NormalNoise,
    roof: NormalNoise,
    factory: PositionalFactory,
}

pub struct IcebergColumn {
    pub x: i32,
    pub z: i32,
    pub height_map_top: i32,
    pub lower_bound: i32,
    pub sea_level: i32,
    pub temperature: f32,
}

impl Iceberg {
    pub fn new(factory: &PositionalFactory) -> Self {
        Self {
            surface: NormalNoise::new(&mut factory.hash_of(SURFACE_NAME), SURFACE_OCTAVE, &[1.0; 3]),
            pillar: NormalNoise::new(&mut factory.hash_of(PILLAR_NAME), PILLAR_OCTAVE, &[1.0; 4]),
            roof: NormalNoise::new(&mut factory.hash_of(ROOF_NAME), ROOF_OCTAVE, &[1.0]),
            factory: *factory,
        }
    }

    pub fn build(&self, palette: &Palette, column: &mut Column, spec: &IcebergColumn) -> bool {
        let (fx, fz) = (spec.x as f32, spec.z as f32);
        let surface = (self.surface.value(fx, 0.0, fz) * SURFACE_SCALE).abs();
        let pillar = self.pillar.value(fx * PILLAR_INPUT, 0.0, fz * PILLAR_INPUT) * PILLAR_SCALE;
        let depth = if surface < pillar { surface } else { pillar };
        if depth <= MIN_HEIGHT {
            return false;
        }
        let min_y = column.min_y();
        let sea = spec.sea_level - min_y;
        let roof = (self.roof.value(fx * ROOF_INPUT, 0.0, fz * ROOF_INPUT) * ROOF_SCALE).abs() * ROOF_HEIGHT_SCALE;
        let mut height = depth * depth * HEIGHT_CURVE;
        let ceiling = roof.ceil() + ROOF_CEILING_OFFSET;
        if ceiling <= height {
            height = ceiling;
        }
        if spec.temperature > WARM_TEMPERATURE {
            height += WARM_LOWERING;
        }
        let solid = SOLID_HEIGHT < height;
        let underwater = if solid { (sea as f32 - height) - UNDERWATER_DEPTH } else { 0.0 };
        let top = if solid { sea as f32 + height } else { 0.0 };
        let mut random = self.factory.at_wide(spec.x, 0, spec.z);
        let max_snow = random.next_int_bounded(SNOW_LIMIT_RANDOM) + SNOW_LIMIT_BASE;
        let snow_floor = random.next_int_bounded(SNOW_FLOOR_RANDOM) + sea + SNOW_FLOOR_ABOVE_SEA;
        let height_map_local = (spec.height_map_top - min_y) as f32;
        let start = top + 1.0;
        let first = (if height_map_local <= start { start } else { height_map_local }) as i32;
        let lower = spec.lower_bound - min_y;
        let top_limit = top as i32;
        let underwater_limit = underwater as i32;
        let mut snow_count = 0;
        for y in (lower..=first).rev() {
            if y >= column.height() {
                continue;
            }
            let block = column.get_local(y);
            let placed = if block == palette.air && y < top_limit {
                random.next_float() > AIR_PLACE_CHANCE
            } else if underwater != 0.0 && block == palette.water && underwater_limit < y && y < sea {
                random.next_float() > WATER_PLACE_CHANCE
            } else {
                false
            };
            if !placed {
                continue;
            }
            if max_snow < snow_count || y <= snow_floor {
                column.set_local(y, palette.packed_ice);
            } else {
                column.set_local(y, palette.snow_block);
                snow_count += 1;
            }
        }
        true
    }
}
