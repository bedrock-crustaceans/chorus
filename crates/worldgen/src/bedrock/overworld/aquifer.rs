use super::noise::NormalNoise;
use super::terrain::{BedrockTerrain, SURFACE_TOP_MARGIN, clamp_unit};
use crate::shared::rng::random::RandomSource;
use crate::shared::rng::xoroshiro::PositionalFactory;
use std::cell::RefCell;
use std::sync::Arc;

const SHORTCUT_TOP: i32 = -55;
const SHORTCUT_LEVEL: i32 = -54;
const NO_FLUID: i32 = -4096;
const CELL_XZ: i32 = 16;
const CELL_Y: i32 = 12;
const JITTER_XZ: i32 = 10;
const JITTER_Y: i32 = 9;
const SEARCH_SHIFT: i32 = 5;
const GRID_SPAN_XZ: i32 = 3;
const SURFACE_OFFSETS: [(i32, i32); 13] = [(-2, -1), (-1, -1), (0, -1), (1, -1), (-3, 0), (-2, 0), (-1, 0), (0, 0), (1, 0), (-2, 1), (-1, 1), (0, 1), (1, 1)];
const SURFACE_UNKNOWN: i32 = 32767;
const SURFACE_MARGIN: i32 = 8;
const POINT_DROP: i32 = 12;
const POINT_RISE: i32 = 4;
const DEEP_DARK_EROSION: f32 = -0.225;
const DEEP_DARK_DEPTH: f32 = 0.9;
const DEPTH_PER_BLOCK: f32 = -0.0078125;
const FLOOD_SPAN_SCALE: f32 = 0.015625;
const FLOODEDNESS_Y_SCALE: f32 = 0.67;
const FLOODED_SLOPE: f32 = 1.1;
const FLOODED_OFFSET: f32 = -0.3;
const DRY_SLOPE: f32 = 1.2;
const DRY_OFFSET: f32 = -0.8;
const SPREAD_CELL_XZ: i32 = 16;
const SPREAD_CELL_Y: i32 = 40;
const SPREAD_Y_DIVISOR: f32 = 1.4;
const SPREAD_NUMERATOR: f32 = 10.0;
const SPREAD_DENOMINATOR: f32 = 3.0;
const LEVEL_STEP: i32 = 3;
const LEVEL_BASE: i32 = 20;
const LAVA_BELOW: i32 = -9;
const LAVA_CELL_XZ: i32 = 64;
const LAVA_CELL_Y: i32 = 40;
const LAVA_THRESHOLD: f32 = 0.3;
const SIMILARITY_SLOPE: f32 = 0.04;
const SCHEDULE_THRESHOLD: f32 = -0.4;
const BARRIER_CUTOFF: f32 = -1.0;
const BARRIER_Y_SCALE: f32 = 0.5;
const PRESSURE_NOISE_LIMIT: f32 = 2.0;
const ABOVE_MIDPOINT_DIVISOR: (f32, f32) = (1.5, 2.5);
const BELOW_MIDPOINT_DIVISOR: (f32, f32) = (3.0, 10.0);
const BELOW_MIDPOINT_OFFSET: f32 = 3.0;
const LEVEL_MIDPOINT: f32 = 0.5;
const DRY_ABOVE_LEVEL: i32 = 5;
const CEILING_REACH: i32 = 80;

pub struct AquiferNoises {
    barrier: NormalNoise,
    floodedness: NormalNoise,
    spread: NormalNoise,
    lava: NormalNoise,
    factory: PositionalFactory,
}

impl AquiferNoises {
    pub fn new(root: &PositionalFactory) -> Self {
        let make = |name: &str, first_octave: i32| NormalNoise::new(&mut root.hash_of(&format!("minecraft:{name}")), first_octave, &[1.0]);
        Self {
            barrier: make("aquifer_barrier", -3),
            floodedness: make("aquifer_fluid_level_floodedness", -7),
            spread: make("aquifer_fluid_level_spread", -5),
            lava: make("aquifer_lava", -1),
            factory: root.hash_of("minecraft:aquifer").fork_positional(),
        }
    }
}

pub use crate::shared::chunk::proto::FluidTick;

pub const HINT_RADIUS: i32 = 5;
const HINT_CELLS: i32 = HINT_RADIUS * 2 + 1;
const HINT_SIDE: i32 = HINT_CELLS * 4;

pub type SurfaceHints = [i16; 16];

#[derive(Clone)]
pub struct SurfaceWindow {
    min_quart_x: i32,
    min_quart_z: i32,
    values: Arc<[i16]>,
}

impl SurfaceWindow {
    pub fn new(cell_x: i32, cell_z: i32, hints: &[Arc<SurfaceHints>]) -> Self {
        assert_eq!(hints.len(), (HINT_CELLS * HINT_CELLS) as usize);
        let mut values = [0i16; (HINT_SIDE * HINT_SIDE) as usize];
        for (index, hint) in hints.iter().enumerate() {
            let (cell_dx, cell_dz) = (index as i32 / HINT_CELLS, index as i32 % HINT_CELLS);
            for (quart, &surface) in hint.iter().enumerate() {
                let (qx, qz) = (cell_dx * 4 + quart as i32 / 4, cell_dz * 4 + quart as i32 % 4);
                values[(qx * HINT_SIDE + qz) as usize] = surface;
            }
        }
        Self {
            min_quart_x: (cell_x - HINT_RADIUS) * 4,
            min_quart_z: (cell_z - HINT_RADIUS) * 4,
            values: Arc::from(&values[..]),
        }
    }

    fn at(&self, quart_x: i32, quart_z: i32) -> i32 {
        let (qx, qz) = (quart_x - self.min_quart_x, quart_z - self.min_quart_z);
        assert!((0..HINT_SIDE).contains(&qx) && (0..HINT_SIDE).contains(&qz), "surface hint requested outside the supplied window");
        self.values[(qx * HINT_SIDE + qz) as usize] as i32
    }
}

#[derive(Clone)]
pub struct Aquifer {
    pub scheduled_ticks: Vec<FluidTick>,
    pub window: SurfaceWindow,
}

#[derive(Clone, Copy, Default)]
struct GridPoint {
    x: i32,
    y: i32,
    z: i32,
}

const NEIGHBOURS: usize = 12;

#[derive(Default)]
struct Neighbourhood {
    key: Option<(i32, i32, i32)>,
    column: Option<(i32, i32)>,
    slots: [usize; NEIGHBOURS],
    points: [GridPoint; NEIGHBOURS],
    horizontal: [i64; NEIGHBOURS],
}

#[derive(Clone, Copy)]
struct FluidStatus {
    level: i32,
    lava: bool,
}

pub struct Substance {
    pub level: i32,
    pub lava: bool,
    pub barrier: f32,
    pub schedule_tick: bool,
}

#[derive(Default)]
struct Slots {
    points: Vec<Option<GridPoint>>,
    statuses: Vec<Option<FluidStatus>>,
}

thread_local! {
    static SLOTS: RefCell<Slots> = RefCell::new(Slots::default());
}

pub struct AquiferSampler<'a> {
    noises: &'a AquiferNoises,
    terrain: &'a BedrockTerrain,
    chunk_x: i32,
    chunk_z: i32,
    points: Vec<Option<GridPoint>>,
    statuses: Vec<Option<FluidStatus>>,
    window: SurfaceWindow,
    ceiling: Option<i32>,
    grid_min_level: i32,
    grid_levels: i32,
    neighbourhood: Neighbourhood,
}

fn similarity(first: u64, second: u64) -> f32 {
    first.abs_diff(second) as i64 as f32 * -SIMILARITY_SLOPE + 1.0
}

fn pressure(noises: &AquiferNoises, x: i32, y: i32, z: i32, first: FluidStatus, second: FluidStatus, noise: &mut Option<f32>) -> f32 {
    if first.level > y && second.level > y && first.lava != second.lava {
        return 1.0;
    }
    if first.level == second.level {
        return 0.0;
    }
    let midpoint = (first.level + second.level) as f32 * LEVEL_MIDPOINT;
    let offset = (y as f32 + LEVEL_MIDPOINT) - midpoint;
    let quotient = (first.level - second.level).abs() as f32 * LEVEL_MIDPOINT - offset.abs();
    let scaled = if offset > 0.0 {
        quotient / if quotient > 0.0 { ABOVE_MIDPOINT_DIVISOR.0 } else { ABOVE_MIDPOINT_DIVISOR.1 }
    } else {
        let shifted = quotient + BELOW_MIDPOINT_OFFSET;
        shifted / if shifted > 0.0 { BELOW_MIDPOINT_DIVISOR.0 } else { BELOW_MIDPOINT_DIVISOR.1 }
    };
    if scaled.abs() > PRESSURE_NOISE_LIMIT {
        return scaled;
    }
    scaled + *noise.get_or_insert_with(|| noises.barrier.value(x as f32, y as f32 * BARRIER_Y_SCALE, z as f32))
}

impl Drop for AquiferSampler<'_> {
    fn drop(&mut self) {
        let slots = Slots {
            points: std::mem::take(&mut self.points),
            statuses: std::mem::take(&mut self.statuses),
        };
        SLOTS.with(|pool| pool.replace(slots));
    }
}

impl<'a> AquiferSampler<'a> {
    pub fn new(noises: &'a AquiferNoises, terrain: &'a BedrockTerrain, window: SurfaceWindow, chunk_x: i32, chunk_z: i32) -> Self {
        let definition = terrain.definition();
        let grid_min_level = (definition.min_y + 1).div_euclid(CELL_Y) - 1;
        let highest = (definition.top() - SURFACE_TOP_MARGIN).max(terrain.sea_level()) + DRY_ABOVE_LEVEL;
        let grid_levels = (highest + 1).div_euclid(CELL_Y) + 1 - grid_min_level + 1;
        let slots = (GRID_SPAN_XZ * GRID_SPAN_XZ * grid_levels) as usize;
        let Slots { mut points, mut statuses } = SLOTS.with(RefCell::take);
        points.clear();
        points.resize(slots, None);
        statuses.clear();
        statuses.resize(slots, None);
        Self {
            noises,
            terrain,
            chunk_x,
            chunk_z,
            points,
            statuses,
            window,
            ceiling: None,
            grid_min_level,
            grid_levels,
            neighbourhood: Neighbourhood::default(),
        }
    }

    fn slot(&self, gx: i32, gy: i32, gz: i32) -> usize {
        let ix = gx - (self.chunk_x - 1);
        let iy = gy - self.grid_min_level;
        let iz = gz - (self.chunk_z - 1);
        debug_assert!((0..GRID_SPAN_XZ).contains(&ix) && (0..self.grid_levels).contains(&iy) && (0..GRID_SPAN_XZ).contains(&iz));
        ((ix * self.grid_levels + iy) * GRID_SPAN_XZ + iz) as usize
    }

    fn point(&mut self, slot: usize, gx: i32, gy: i32, gz: i32) -> GridPoint {
        *self.points[slot].get_or_insert_with(|| {
            let mut random = self.noises.factory.at(gx, gy, gz);
            let x = gx * CELL_XZ + random.next_int_bounded(JITTER_XZ);
            let y = gy * CELL_Y + random.next_int_bounded(JITTER_Y);
            let z = gz * CELL_XZ + random.next_int_bounded(JITTER_XZ);
            GridPoint { x, y, z }
        })
    }

    fn preliminary_surface(&self, quart_x: i32, quart_z: i32) -> i32 {
        self.window.at(quart_x, quart_z)
    }

    fn ceiling(&mut self) -> i32 {
        if let Some(ceiling) = self.ceiling {
            return ceiling;
        }
        let (min_x, min_z) = (self.chunk_x * 16, self.chunk_z * 16);
        let mut highest = self.terrain.sea_level();
        for quart_x in ((min_x - CEILING_REACH) >> 2)..=((min_x + CEILING_REACH) >> 2) {
            for quart_z in ((min_z - CEILING_REACH) >> 2)..=((min_z + CEILING_REACH) >> 2) {
                highest = highest.max(self.preliminary_surface(quart_x, quart_z));
            }
        }
        self.ceiling = Some(highest);
        highest
    }

    fn status(&mut self, slot: usize) -> FluidStatus {
        if let Some(status) = self.statuses[slot] {
            return status;
        }
        let point = self.points[slot].expect("status requested for a point that was never created");
        let status = self.compute_status(point);
        self.statuses[slot] = Some(status);
        status
    }

    fn compute_status(&mut self, point: GridPoint) -> FluidStatus {
        let flooded = FluidStatus {
            level: self.terrain.sea_level(),
            lava: false,
        };
        let mut min_surface = SURFACE_UNKNOWN;
        let mut flagged = false;
        for (offset_x, offset_z) in SURFACE_OFFSETS {
            let surface = self.preliminary_surface((point.x + CELL_XZ * offset_x) >> 2, (point.z + CELL_XZ * offset_z) >> 2);
            let center = offset_x == 0 && offset_z == 0;
            if center && surface + SURFACE_MARGIN < point.y - POINT_DROP {
                return flooded;
            }
            if (center || point.y + POINT_RISE > surface) && surface + SURFACE_MARGIN < self.terrain.sea_level() {
                if surface < point.y + POINT_RISE {
                    return flooded;
                }
                flagged |= center;
            }
            min_surface = min_surface.min(surface);
        }

        let (erosion, offset) = self.terrain.erosion_and_offset(point.x >> 2, point.z >> 2);
        if erosion < DEEP_DARK_EROSION && point.y as f32 * DEPTH_PER_BLOCK + 1.0 + offset + self.terrain.global_offset() > DEEP_DARK_DEPTH {
            return FluidStatus { level: NO_FLUID, lava: false };
        }

        let mut dryness = 1.0f32;
        if flagged {
            let span = (min_surface + SURFACE_MARGIN - point.y) as f32 * FLOOD_SPAN_SCALE;
            dryness = if span < 0.0 {
                -0.0
            } else if span <= 1.0 {
                -((1.0 - span) + -1.0)
            } else {
                1.0
            };
        }
        let floodedness = clamp_unit(self.noises.floodedness.value(point.x as f32, point.y as f32 * FLOODEDNESS_Y_SCALE, point.z as f32));
        if floodedness > dryness * FLOODED_SLOPE + FLOODED_OFFSET {
            return flooded;
        }
        if floodedness < dryness * DRY_SLOPE + DRY_OFFSET {
            return FluidStatus { level: NO_FLUID, lava: false };
        }

        let spread_x = point.x.div_euclid(SPREAD_CELL_XZ);
        let spread_y = point.y.div_euclid(SPREAD_CELL_Y);
        let spread_z = point.z.div_euclid(SPREAD_CELL_XZ);
        let spread = self.noises.spread.value(spread_x as f32, spread_y as f32 / SPREAD_Y_DIVISOR, spread_z as f32);
        let scaled = (spread * SPREAD_NUMERATOR) / SPREAD_DENOMINATOR;
        let level = scaled.floor() as i32 * LEVEL_STEP + spread_y * SPREAD_CELL_Y + LEVEL_BASE;
        let lava = level < LAVA_BELOW
            && self
                .noises
                .lava
                .value(point.x.div_euclid(LAVA_CELL_XZ) as f32, point.y.div_euclid(LAVA_CELL_Y) as f32, point.z.div_euclid(LAVA_CELL_XZ) as f32)
                .abs()
                > LAVA_THRESHOLD;
        FluidStatus { level: level.min(min_surface), lava }
    }

    pub fn dry_above(&mut self) -> i32 {
        (self.terrain.sea_level() + DRY_ABOVE_LEVEL).max(self.ceiling() + DRY_ABOVE_LEVEL)
    }

    pub fn substance(&mut self, x: i32, y: i32, z: i32) -> Substance {
        if y <= SHORTCUT_TOP {
            return Substance {
                level: SHORTCUT_LEVEL,
                lava: true,
                barrier: 0.0,
                schedule_tick: y == SHORTCUT_TOP,
            };
        }
        if !self.terrain.aquifers_enabled() {
            return Substance {
                level: self.terrain.sea_level(),
                lava: false,
                barrier: 0.0,
                schedule_tick: false,
            };
        }
        if y >= self.terrain.sea_level() + DRY_ABOVE_LEVEL && y >= self.ceiling() + DRY_ABOVE_LEVEL {
            return Substance {
                level: NO_FLUID,
                lava: false,
                barrier: 0.0,
                schedule_tick: false,
            };
        }
        let grid_x = (x - SEARCH_SHIFT).div_euclid(CELL_XZ);
        let grid_y = (y + 1).div_euclid(CELL_Y);
        let grid_z = (z - SEARCH_SHIFT).div_euclid(CELL_XZ);
        let key = (grid_x, grid_y, grid_z);
        if self.neighbourhood.key != Some(key) {
            let mut index = 0;
            for step_x in 0..2 {
                for step_y in -1..=1 {
                    for step_z in 0..2 {
                        let (gx, gy, gz) = (grid_x + step_x, grid_y + step_y, grid_z + step_z);
                        let slot = self.slot(gx, gy, gz);
                        self.neighbourhood.points[index] = self.point(slot, gx, gy, gz);
                        self.neighbourhood.slots[index] = slot;
                        index += 1;
                    }
                }
            }
            self.neighbourhood.key = Some(key);
            self.neighbourhood.column = None;
        }
        if self.neighbourhood.column != Some((x, z)) {
            for index in 0..NEIGHBOURS {
                let point = self.neighbourhood.points[index];
                let (dx, dz) = ((point.x - x) as i64, (point.z - z) as i64);
                self.neighbourhood.horizontal[index] = dx * dx + dz * dz;
            }
            self.neighbourhood.column = Some((x, z));
        }
        let mut closest = [(u64::MAX, 0usize); 3];
        for index in 0..NEIGHBOURS {
            let dy = (self.neighbourhood.points[index].y - y) as i64;
            let distance = (self.neighbourhood.horizontal[index] + dy * dy) as u64;
            let slot = self.neighbourhood.slots[index];
            if closest[0].0 >= distance {
                closest[2] = closest[1];
                closest[1] = closest[0];
                closest[0] = (distance, slot);
            } else if closest[1].0 >= distance {
                closest[2] = closest[1];
                closest[1] = (distance, slot);
            } else if closest[2].0 >= distance {
                closest[2] = (distance, slot);
            }
        }
        let [(d1, k1), (d2, k2), (d3, k3)] = closest;
        let near_similarity = similarity(d1, d2);
        let first = self.status(k1);
        let schedule_tick = near_similarity > SCHEDULE_THRESHOLD;
        if near_similarity <= BARRIER_CUTOFF {
            return Substance {
                level: first.level,
                lava: first.lava,
                barrier: 0.0,
                schedule_tick,
            };
        }
        let second = self.status(k2);
        let third = self.status(k3);
        let mut noise = None;
        let pressure_12 = pressure(self.noises, x, y, z, first, second, &mut noise);
        let pressure_13 = pressure(self.noises, x, y, z, first, third, &mut noise);
        let pressure_23 = pressure(self.noises, x, y, z, second, third, &mut noise);
        let weight_13 = similarity(d3, d1).max(0.0);
        let weight_23 = similarity(d3, d2).max(0.0);
        let strongest = pressure_12.max((weight_13 * pressure_13).max(weight_23 * pressure_23));
        let near = near_similarity.max(0.0);
        Substance {
            level: first.level,
            lava: first.lava,
            barrier: ((near + near) * strongest).max(0.0),
            schedule_tick,
        }
    }
}
