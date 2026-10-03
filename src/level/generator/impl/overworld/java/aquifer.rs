use std::collections::HashMap;

use rustc_hash::FxBuildHasher;

use super::random::{PositionalFactory, RandomSource};
use super::terrain::Terrain;

const WAY_BELOW_MIN_Y: i32 = -2032 << 4;
const LAVA_LEVEL: i32 = -54;
const SURFACE_SAMPLING_OFFSETS: [(i32, i32); 13] = [(0, 0), (-2, -1), (-1, -1), (0, -1), (1, -1), (-3, 0), (-2, 0), (-1, 0), (1, 0), (-2, 1), (-1, 1), (0, 1), (1, 1)];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fluid {
    Air,
    Water,
    Lava,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FluidStatus {
    level: i32,
    fluid: Fluid,
}

impl FluidStatus {
    fn at(self, y: i32) -> Fluid {
        if y < self.level { self.fluid } else { Fluid::Air }
    }
}

fn global_fluid(sea_level: i32, y: i32) -> FluidStatus {
    if y < LAVA_LEVEL.min(sea_level) {
        FluidStatus {
            level: LAVA_LEVEL,
            fluid: Fluid::Lava,
        }
    } else {
        FluidStatus {
            level: sea_level,
            fluid: Fluid::Water,
        }
    }
}

fn similarity(a: i32, b: i32) -> f64 {
    1.0 - (b - a) as f64 / 25.0
}

fn grid_y(y: i32) -> i32 {
    y.div_euclid(12)
}

type Neighbours = ((i32, i32, i32), [(usize, (i32, i32, i32)); 12]);

#[derive(Clone)]
pub struct Aquifer {
    random: PositionalFactory,
    sea_level: i32,
    min_grid_x: i32,
    min_grid_y: i32,
    min_grid_z: i32,
    size_x: i32,
    size_z: i32,
    locations: Vec<Option<(i32, i32, i32)>>,
    statuses: Vec<Option<FluidStatus>>,
    surface_levels: HashMap<(i32, i32), i32, FxBuildHasher>,
    skip_sampling_above_y: i32,
    neighbours: Option<Neighbours>,
}

impl Aquifer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        terrain: &Terrain,
        random: PositionalFactory,
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        height: i32,
        sea_level: i32,
        known_surface_levels: impl IntoIterator<Item = ((i32, i32), i32)>,
    ) -> Self {
        let (min_x, min_z) = (chunk_x << 4, chunk_z << 4);
        let (max_x, max_y, max_z) = (min_x + 15, min_y + height - 1, min_z + 15);
        let min_grid_x = (min_x - 5) >> 4;
        let max_grid_x = ((max_x - 5) >> 4) + 1;
        let min_grid_y = grid_y(min_y + 1) - 1;
        let max_grid_y = grid_y(max_y + 1) + 1;
        let min_grid_z = (min_z - 5) >> 4;
        let max_grid_z = ((max_z - 5) >> 4) + 1;
        let size_x = max_grid_x - min_grid_x + 1;
        let size_y = max_grid_y - min_grid_y + 1;
        let size_z = max_grid_z - min_grid_z + 1;
        let total = (size_x * size_y * size_z) as usize;
        let mut aquifer = Self {
            random,
            sea_level,
            min_grid_x,
            min_grid_y,
            min_grid_z,
            size_x,
            size_z,
            locations: vec![None; total],
            statuses: vec![None; total],
            surface_levels: known_surface_levels.into_iter().collect(),
            skip_sampling_above_y: 0,
            neighbours: None,
        };
        let max_surface = aquifer.max_surface_level(terrain, min_grid_x << 4, min_grid_z << 4, (max_grid_x << 4) + 9, (max_grid_z << 4) + 9) + 8;
        let skip_grid_y = grid_y(max_surface + 12) + 1;
        aquifer.skip_sampling_above_y = skip_grid_y * 12 + 11 - 1;
        aquifer
    }

    fn surface_level(&mut self, terrain: &Terrain, x: i32, z: i32) -> i32 {
        let (qx, qz) = ((x >> 2) << 2, (z >> 2) << 2);
        if let Some(&level) = self.surface_levels.get(&(qx, qz)) {
            return level;
        }
        let level = terrain.preliminary_surface_level(qx, qz).floor() as i32;
        self.surface_levels.insert((qx, qz), level);
        level
    }

    fn max_surface_level(&mut self, terrain: &Terrain, min_x: i32, min_z: i32, max_x: i32, max_z: i32) -> i32 {
        let mut max = i32::MIN;
        for qz in (min_z >> 2)..=(max_z >> 2) {
            for qx in (min_x >> 2)..=(max_x >> 2) {
                max = max.max(self.surface_level(terrain, qx << 2, qz << 2));
            }
        }
        max
    }

    fn index(&self, gx: i32, gy: i32, gz: i32) -> usize {
        let (x, y, z) = (gx - self.min_grid_x, gy - self.min_grid_y, gz - self.min_grid_z);
        ((y * self.size_z + z) * self.size_x + x) as usize
    }

    fn location(&mut self, gx: i32, gy: i32, gz: i32) -> (usize, (i32, i32, i32)) {
        let index = self.index(gx, gy, gz);
        if let Some(location) = self.locations[index] {
            return (index, location);
        }
        let mut random = self.random.at(gx, gy, gz);
        let location = ((gx << 4) + random.next_int_bounded(10), gy * 12 + random.next_int_bounded(9), (gz << 4) + random.next_int_bounded(10));
        self.locations[index] = Some(location);
        (index, location)
    }

    fn neighbours(&mut self, cell: (i32, i32, i32)) -> &[(usize, (i32, i32, i32)); 12] {
        if self.neighbours.is_some_and(|(cached, _)| cached == cell) {
            return &self.neighbours.as_ref().expect("checked above").1;
        }
        let (ax, ay, az) = cell;
        let mut neighbours = [(0, (0, 0, 0)); 12];
        let mut slots = neighbours.iter_mut();
        for x1 in 0..=1 {
            for y1 in -1..=1 {
                for z1 in 0..=1 {
                    *slots.next().expect("twelve neighbours") = self.location(ax + x1, ay + y1, az + z1);
                }
            }
        }
        self.neighbours = Some((cell, neighbours));
        &self.neighbours.as_ref().expect("just cached").1
    }

    fn status(&mut self, terrain: &Terrain, index: usize) -> FluidStatus {
        if let Some(status) = self.statuses[index] {
            return status;
        }
        let (x, y, z) = self.locations[index].expect("aquifer location computed before its status");
        let status = self.compute_fluid(terrain, x, y, z);
        self.statuses[index] = Some(status);
        status
    }

    #[inline]
    pub fn compute_substance(&mut self, terrain: &Terrain, x: i32, y: i32, z: i32, density: f64) -> Option<Fluid> {
        if density > 0.0 {
            return None;
        }
        let global = global_fluid(self.sea_level, y);
        if y > self.skip_sampling_above_y {
            return Some(global.at(y));
        }
        if global.at(y) == Fluid::Lava {
            return Some(Fluid::Lava);
        }
        self.local_substance(terrain, x, y, z, density)
    }

    #[inline(never)]
    fn local_substance(&mut self, terrain: &Terrain, x: i32, y: i32, z: i32, density: f64) -> Option<Fluid> {
        let mut distances = [i32::MAX; 3];
        let mut closest = [0usize; 3];
        for &(index, (lx, ly, lz)) in self.neighbours(((x - 5) >> 4, grid_y(y + 1), (z - 5) >> 4)) {
            let (dx, dy, dz) = (lx - x, ly - y, lz - z);
            let distance = dx * dx + dy * dy + dz * dz;
            if distances[0] >= distance {
                distances = [distance, distances[0], distances[1]];
                closest = [index, closest[0], closest[1]];
            } else if distances[1] >= distance {
                distances = [distances[0], distance, distances[1]];
                closest = [closest[0], index, closest[1]];
            } else if distances[2] >= distance {
                distances[2] = distance;
                closest[2] = index;
            }
        }

        let status1 = self.status(terrain, closest[0]);
        let similarity12 = similarity(distances[0], distances[1]);
        let fluid = status1.at(y);
        if similarity12 <= 0.0 {
            return Some(fluid);
        }
        if fluid == Fluid::Water && global_fluid(self.sea_level, y - 1).at(y - 1) == Fluid::Lava {
            return Some(fluid);
        }

        let mut barrier = f64::NAN;
        let status2 = self.status(terrain, closest[1]);
        if density + similarity12 * self.pressure(terrain, x, y, z, &mut barrier, status1, status2) > 0.0 {
            return None;
        }
        let status3 = self.status(terrain, closest[2]);
        let similarity13 = similarity(distances[0], distances[2]);
        if similarity13 > 0.0 && density + similarity12 * similarity13 * self.pressure(terrain, x, y, z, &mut barrier, status1, status3) > 0.0 {
            return None;
        }
        let similarity23 = similarity(distances[1], distances[2]);
        if similarity23 > 0.0 && density + similarity12 * similarity23 * self.pressure(terrain, x, y, z, &mut barrier, status2, status3) > 0.0 {
            return None;
        }
        Some(fluid)
    }

    #[allow(clippy::too_many_arguments)]
    fn pressure(&self, terrain: &Terrain, x: i32, y: i32, z: i32, barrier: &mut f64, first: FluidStatus, second: FluidStatus) -> f64 {
        let (a, b) = (first.at(y), second.at(y));
        if (a == Fluid::Lava && b == Fluid::Water) || (a == Fluid::Water && b == Fluid::Lava) {
            return 2.0;
        }
        let level_diff = (first.level - second.level).abs();
        if level_diff == 0 {
            return 0.0;
        }
        let average = 0.5 * (first.level + second.level) as f64;
        let above_average = y as f64 + 0.5 - average;
        let towards_middle = level_diff as f64 / 2.0 - above_average.abs();
        let gradient = if above_average > 0.0 {
            if towards_middle > 0.0 { towards_middle / 1.5 } else { towards_middle / 2.5 }
        } else {
            let center = 3.0 + towards_middle;
            if center > 0.0 { center / 3.0 } else { center / 10.0 }
        };
        let noise = if (-2.0..=2.0).contains(&gradient) {
            if barrier.is_nan() {
                *barrier = terrain.aquifer_barrier(x, y, z) as f64;
            }
            *barrier
        } else {
            0.0
        };
        2.0 * (noise + gradient)
    }

    fn compute_fluid(&mut self, terrain: &Terrain, x: i32, y: i32, z: i32) -> FluidStatus {
        let global = global_fluid(self.sea_level, y);
        let mut lowest_surface = i32::MAX;
        let top = y + 12;
        let bottom = y - 12;
        let mut center_under_global = false;
        for (ox, oz) in SURFACE_SAMPLING_OFFSETS {
            let (sx, sz) = (x + (ox << 4), z + (oz << 4));
            let surface = self.surface_level(terrain, sx, sz);
            let adjusted = surface + 8;
            let start = ox == 0 && oz == 0;
            if start && bottom > adjusted {
                return global;
            }
            let pokes_above = top > adjusted;
            if pokes_above || start {
                let at_surface = global_fluid(self.sea_level, adjusted);
                if at_surface.at(adjusted) != Fluid::Air {
                    if start {
                        center_under_global = true;
                    }
                    if pokes_above {
                        return at_surface;
                    }
                }
            }
            lowest_surface = lowest_surface.min(surface);
        }
        let level = self.fluid_surface_level(terrain, x, y, z, global, lowest_surface, center_under_global);
        FluidStatus {
            level,
            fluid: self.fluid_type(terrain, x, y, z, global, level),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn fluid_surface_level(&mut self, terrain: &Terrain, x: i32, y: i32, z: i32, global: FluidStatus, lowest_surface: i32, center_under_global: bool) -> i32 {
        let (partially, fully) = if terrain.aquifer_exclusion(x, y, z) > 0.0 {
            (-1.0, -1.0)
        } else {
            let below_surface = (lowest_surface + 8 - y) as f64;
            let factor = if center_under_global { (1.0 - below_surface / 64.0).clamp(0.0, 1.0) } else { 0.0 };
            let noise = (terrain.aquifer_floodedness(x, y, z) as f64).clamp(-1.0, 1.0);
            let map = |from: f64, to: f64| from + (1.0 - factor) * (to - from);
            (noise - map(-0.8, 0.4), noise - map(-0.3, 0.8))
        };
        if fully > 0.0 {
            global.level
        } else if partially > 0.0 {
            let (cx, cy, cz) = (x.div_euclid(16), y.div_euclid(40), z.div_euclid(16));
            let spread = terrain.aquifer_spread(cx, cy, cz) as f64 * 10.0;
            let quantized = (spread / 3.0).floor() as i32 * 3;
            lowest_surface.min(cy * 40 + 20 + quantized)
        } else {
            WAY_BELOW_MIN_Y
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn fluid_type(&mut self, terrain: &Terrain, x: i32, y: i32, z: i32, global: FluidStatus, level: i32) -> Fluid {
        if level <= -10 && level != WAY_BELOW_MIN_Y && global.fluid != Fluid::Lava {
            let lava = terrain.aquifer_lava(x.div_euclid(64), y.div_euclid(40), z.div_euclid(64));
            if lava.abs() > 0.3 {
                return Fluid::Lava;
            }
        }
        global.fluid
    }
}
