use super::noise::NormalNoise;
use super::perlin::{GRID_COLUMNS, TerrainBuffers};
use super::terrain::{BedrockTerrain, clamp_unit};
use crate::shared::rng::xoroshiro::PositionalFactory;
use crate::shared::terrain::{CELL_HEIGHT, CELL_WIDTH, Corner, CornerColumns};
use std::cell::RefCell;

const BUFFER_DENSITY_FLOOR: f32 = -150.0;
const UNSAMPLED_BUFFER: f32 = 1.0;
const TOP_SLIDE_FLOOR: f32 = -10.0;
const RIDGE_SCALE: f32 = 2.6666667;
const VEININESS_SCALE: f32 = 1.5;
const VEIN_SCALE: f32 = 4.0;

pub struct BufferNoises {
    noodle: NormalNoise,
    noodle_thickness: NormalNoise,
    noodle_ridge_a: NormalNoise,
    noodle_ridge_b: NormalNoise,
    veininess: NormalNoise,
    vein_a: NormalNoise,
    vein_b: NormalNoise,
}

impl BufferNoises {
    pub fn new(factory: &PositionalFactory) -> Self {
        let make = |name: &str, first_octave: i32| NormalNoise::new(&mut factory.hash_of(&format!("minecraft:{name}")), first_octave, &[1.0]);
        Self {
            noodle: make("noodle", -8),
            noodle_thickness: make("noodle_thickness", -8),
            noodle_ridge_a: make("noodle_ridge_a", -7),
            noodle_ridge_b: make("noodle_ridge_b", -7),
            veininess: make("ore_veininess", -8),
            vein_a: make("ore_vein_a", -7),
            vein_b: make("ore_vein_b", -7),
        }
    }

    fn corner(&self, density: f32, x: f32, y: f32, z: f32) -> Corner {
        if density < BUFFER_DENSITY_FLOOR {
            return unsampled(density);
        }
        Corner {
            main: density,
            noodle: clamp_unit(self.noodle.value(x, y, z)),
            noodle_thickness: clamp_unit(self.noodle_thickness.value(x, y, z)),
            noodle_ridge_a: clamp_unit(self.noodle_ridge_a.value(x * RIDGE_SCALE, y * RIDGE_SCALE, z * RIDGE_SCALE)),
            noodle_ridge_b: clamp_unit(self.noodle_ridge_b.value(x * RIDGE_SCALE, y * RIDGE_SCALE, z * RIDGE_SCALE)),
            vein_toggle: clamp_unit(self.veininess.value(x * VEININESS_SCALE, y * VEININESS_SCALE, z * VEININESS_SCALE)),
            vein_a: clamp_unit(self.vein_a.value(x * VEIN_SCALE, y * VEIN_SCALE, z * VEIN_SCALE)),
            vein_b: clamp_unit(self.vein_b.value(x * VEIN_SCALE, y * VEIN_SCALE, z * VEIN_SCALE)),
        }
    }
}

fn unsampled(density: f32) -> Corner {
    Corner {
        main: density,
        noodle: UNSAMPLED_BUFFER,
        noodle_thickness: UNSAMPLED_BUFFER,
        noodle_ridge_a: UNSAMPLED_BUFFER,
        noodle_ridge_b: UNSAMPLED_BUFFER,
        vein_toggle: UNSAMPLED_BUFFER,
        vein_a: UNSAMPLED_BUFFER,
        vein_b: UNSAMPLED_BUFFER,
    }
}

struct Work {
    buffers: TerrainBuffers,
    densities: Vec<f32>,
}

thread_local! {
    static WORK: RefCell<Work> = RefCell::new(Work { buffers: TerrainBuffers::new(), densities: Vec::new() });
}

pub fn corner_columns(terrain: &BedrockTerrain, noises: &BufferNoises, chunk_x: i32, chunk_z: i32) -> CornerColumns {
    let columns = CornerColumns::COLUMNS;
    let levels = terrain.corner_levels();
    let noise_levels = terrain.noise_levels();
    let min_y = terrain.min_y();
    let mut corners = vec![unsampled(TOP_SLIDE_FLOOR); (columns * columns) as usize * levels];
    WORK.with(|work| {
        let work = &mut *work.borrow_mut();
        terrain.density_corners(chunk_x, chunk_z, &mut work.buffers, &mut work.densities);
        for cz in 0..columns {
            for cx in 0..columns {
                let column = &mut corners[((cz * columns + cx) as usize) * levels..][..levels];
                let source = (cx as usize * GRID_COLUMNS + cz as usize) * noise_levels;
                let x = (chunk_x * 16 + cx * CELL_WIDTH) as f32;
                let z = (chunk_z * 16 + cz * CELL_WIDTH) as f32;
                for (cy, corner) in column.iter_mut().take(noise_levels).enumerate() {
                    let y = (min_y + cy as i32 * CELL_HEIGHT) as f32;
                    *corner = noises.corner(work.densities[source + cy], x, y, z);
                }
            }
        }
    });
    CornerColumns::from_corners(corners, levels)
}
