mod column;
pub(in crate::bedrock::overworld) mod data;
mod default;
mod gradient;
mod iceberg;
mod mesa;
mod palette;
mod simplex;
mod temperature;
#[cfg(test)]
mod tests;

use super::terrain::{BedrockTerrain, SURFACE_NOT_FOUND};
use crate::shared::biome::Biome;
use crate::shared::blocks::BlockTable;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::rng::seeds::{chunk_random, level_seed32, material_adjustment_random};
use crate::shared::rng::xoroshiro::{PositionalFactory, Xoroshiro};
use column::Column;
use data::{Builder, SurfaceData};
use default::DefaultColumn;
use gradient::Segment;
use iceberg::{Iceberg, IcebergColumn};
use mesa::{Mesa, MesaColumn};
use palette::Palette;
use simplex::{PerlinSimplex, REGION_WIDTH};
use temperature::Temperature;
pub(in crate::bedrock::overworld) use temperature::{info_noise, temperature_at};

const SURFACE_NAME: &str = "minecraft:surface";
const SURFACE_LEVELS: usize = 4;
const MATERIAL_LEVELS: usize = 3;
const SWAMP_LEVELS: usize = 2;
const MATERIAL_NORMALIZATION: f32 = 1.0 / 7.0;
const SWAMP_SCALE: f32 = 0.25;
const BEDROCK_LAYERS_RANDOM: i32 = 4;
const BEDROCK_LAYERS_BASE: i32 = 2;
const RUN_DEPTH_RANDOM: f32 = 0.25;
const DEPTH_DIVISOR: f32 = 3.0;
const RUN_DEPTH_BASE: f32 = 3.0;
const LOWER_BOUND_MARGIN: i32 = 8;
const STEEP_DIFFERENCE: i32 = 3;
const LAST_INNER_COLUMN: i32 = 14;
const COLUMNS: i32 = 16;
const QUART_SHIFT: i32 = 2;
const SURFACE_TEMPERATURE_Y: i32 = 0;

pub type ColumnValues = [i32; (COLUMNS * COLUMNS) as usize];

pub struct SurfacePass {
    palette: Palette,
    data: SurfaceData,
    depth: PerlinSimplex,
    adjustment: PerlinSimplex,
    swamp: PerlinSimplex,
    temperature: Temperature,
    iceberg: Iceberg,
    mesa: Mesa,
}

fn column_index(x: i32, z: i32) -> usize {
    (z * COLUMNS + x) as usize
}

impl SurfacePass {
    pub fn new(seed: i64, factory: &PositionalFactory, table: &mut BlockTable) -> Self {
        Self {
            palette: Palette::new(table),
            data: SurfaceData::parse(table, factory),
            depth: PerlinSimplex::new(&mut factory.hash_of(SURFACE_NAME), SURFACE_LEVELS, true),
            adjustment: PerlinSimplex::new(&mut material_adjustment_random(seed), MATERIAL_LEVELS, false),
            swamp: PerlinSimplex::new(&mut MersenneTwister::new(level_seed32(seed) as u32), SWAMP_LEVELS, false),
            temperature: Temperature::new(),
            iceberg: Iceberg::new(&Xoroshiro::new(level_seed32(seed) as i64).fork_positional()),
            mesa: Mesa::new(seed),
        }
    }

    fn run_depth(random: &mut MersenneTwister, depth: f32) -> i32 {
        (random.next_float() * RUN_DEPTH_RANDOM + (depth / DEPTH_DIVISOR + RUN_DEPTH_BASE)) as i32
    }

    fn is_steep(&self, chunk: &ProtoChunk, tops: &ColumnValues, lx: i32, lz: i32, steep: &data::Steep) -> bool {
        let min_y = chunk.min_y;
        let height_at = |x: i32, z: i32| {
            let mut y = (tops[column_index(x, z)] - min_y).min(chunk.height - 1);
            while y > 0 {
                if chunk.get(x, min_y + y, z) != self.palette.air {
                    return y;
                }
                y -= 1;
            }
            0
        };
        if steep.north || steep.south {
            let difference = height_at(lx, (lz - 1).max(0)) - height_at(lx, lz.min(LAST_INNER_COLUMN) + 1);
            if (steep.south && difference > STEEP_DIFFERENCE) || (steep.north && difference < -STEEP_DIFFERENCE) {
                return true;
            }
        }
        if steep.west || steep.east {
            let difference = height_at(lx.max(1) - 1, lz) - height_at(lx.min(LAST_INNER_COLUMN) + 1, lz);
            if (steep.east && difference > STEEP_DIFFERENCE) || (steep.west && difference < -STEEP_DIFFERENCE) {
                return true;
            }
        }
        false
    }

    fn swamp_puddle(&self, column: &mut Column, x: i32, z: i32, top: i32, puddle_depth: i32, sea_level: i32) {
        if self.swamp.sum_2d(x as f32 * SWAMP_SCALE, z as f32 * SWAMP_SCALE) <= 0.0 || sea_level < top {
            return;
        }
        for y in (sea_level - puddle_depth..top).rev() {
            if column.get(y) != self.palette.water {
                column.set(y, self.palette.water);
                break;
            }
        }
    }

    pub fn build(&self, terrain: &BedrockTerrain, chunk: &mut ProtoChunk) {
        let lower_bounds = self.build_columns(terrain, chunk);
        self.build_gradient_segments(chunk, &lower_bounds);
    }

    pub fn build_columns(&self, terrain: &BedrockTerrain, chunk: &mut ProtoChunk) -> ColumnValues {
        let (chunk_x, chunk_z) = (chunk.x, chunk.z);
        let min_y = chunk.min_y;
        let max_y = chunk.max_y();
        let sea_level = terrain.sea_level();
        let depth = self.depth.region(chunk_x, chunk_z);
        let mut random = chunk_random(chunk_x, chunk_z);
        let tops: ColumnValues = std::array::from_fn(|index| chunk.top(index as i32 % COLUMNS, index as i32 / COLUMNS));
        let preliminary: [i32; 16] = std::array::from_fn(|index| terrain.preliminary_surface(chunk_x * 4 + index as i32 / 4, chunk_z * 4 + index as i32 % 4));
        let mut lower_bounds: ColumnValues = [min_y; (COLUMNS * COLUMNS) as usize];
        for lx in 0..COLUMNS {
            for lz in 0..COLUMNS {
                let layers = random.next_int_bounded(BEDROCK_LAYERS_RANDOM) + BEDROCK_LAYERS_BASE;
                for offset in 0..layers {
                    chunk.set(lx, min_y + offset, lz, self.palette.bedrock);
                }
            }
        }
        for lx in 0..COLUMNS {
            for lz in 0..COLUMNS {
                let index = column_index(lx, lz);
                let top = tops[index];
                let Some(surface) = self.data.get(&chunk.biome_at(lx, top, lz)) else { continue };
                let (x, z) = (chunk.min_x() + lx, chunk.min_z() + lz);
                let (fx, fz) = (x as f32, z as f32);
                let preliminary_surface = preliminary[((lx >> QUART_SHIFT) * 4 + (lz >> QUART_SHIFT)) as usize];
                let lower_bound = if preliminary_surface == SURFACE_NOT_FOUND {
                    min_y
                } else {
                    (preliminary_surface - LOWER_BOUND_MARGIN).max(min_y).min(max_y)
                };
                lower_bounds[index] = lower_bound;
                let mut materials = surface.materials;
                for adjustment in &surface.adjustments {
                    let noise = self.adjustment.sum_3d(fx * adjustment.scale, 0.0 * adjustment.scale, fz * adjustment.scale) * MATERIAL_NORMALIZATION;
                    if adjustment.low <= noise && noise < adjustment.high {
                        adjustment.apply(&mut materials);
                    }
                }
                let steep = surface.steep.as_ref().filter(|steep| self.is_steep(chunk, &tops, lx, lz, steep)).map(|steep| steep.block);
                let start_y = (min_y + chunk.height - 1).min(top);
                let temperature = self.temperature.at(surface, x, SURFACE_TEMPERATURE_Y, z, sea_level);
                let depth_value = depth[lz as usize * REGION_WIDTH + lx as usize];
                let mut column = Column::new(chunk, lx, lz);
                let default_column = |run_depth: i32| DefaultColumn {
                    start_y,
                    lower_bound,
                    run_depth,
                    materials,
                    steep,
                    sea_floor_depth: surface.sea_floor_depth,
                    temperature,
                };
                match &surface.builder {
                    Builder::Overworld => {
                        let spec = default_column(Self::run_depth(&mut random, depth_value));
                        default::build(&self.palette, &mut column, &spec, &mut random);
                    }
                    Builder::Swamp { puddle_depth } => {
                        self.swamp_puddle(&mut column, x, z, top, *puddle_depth, sea_level);
                        let spec = default_column(Self::run_depth(&mut random, depth_value));
                        default::build(&self.palette, &mut column, &spec, &mut random);
                    }
                    Builder::FrozenOcean => {
                        let iceberg = IcebergColumn {
                            x,
                            z,
                            height_map_top: top,
                            lower_bound,
                            sea_level,
                            temperature,
                        };
                        if self.iceberg.build(&self.palette, &mut column, &iceberg) {
                            let spec = default_column(Self::run_depth(&mut random, depth_value));
                            default::build(&self.palette, &mut column, &spec, &mut random);
                        }
                    }
                    Builder::Mesa(settings) => {
                        let spec = MesaColumn {
                            x,
                            z,
                            lower_bound,
                            sea_level,
                            depth: depth_value,
                            materials,
                            settings,
                        };
                        self.mesa.build(&self.palette, &mut column, &spec, &mut random);
                    }
                    Builder::NoiseGradient(_) => {}
                }
            }
        }
        lower_bounds
    }

    pub fn build_gradient_segments(&self, chunk: &mut ProtoChunk, lower_bounds: &ColumnValues) {
        let min_y = chunk.min_y;
        for lx in 0..COLUMNS {
            for lz in 0..COLUMNS {
                let top = lower_bounds[column_index(lx, lz)];
                let mut first = min_y;
                while first <= top {
                    let biome_id = chunk.biome_id_at(lx, first, lz);
                    let mut last = first;
                    while last < top && chunk.biome_id_at(lx, last + 1, lz) == biome_id {
                        last += 1;
                    }
                    let biome = Biome::generated(biome_id);
                    if let Some(Builder::NoiseGradient(settings)) = self.data.get(&biome).map(|surface| &surface.builder) {
                        let segment = Segment {
                            x: chunk.min_x() + lx,
                            z: chunk.min_z() + lz,
                            first,
                            last,
                        };
                        gradient::build(&self.palette, &mut Column::new(chunk, lx, lz), settings, &segment);
                    }
                    first = last + 1;
                }
            }
        }
    }
}
