use super::aquifer::{Aquifer, AquiferNoises, AquiferSampler, FluidTick, SurfaceWindow};
use super::noise::NormalNoise;
use super::terrain::BedrockTerrain;
use crate::shared::blocks::{AIR, BlockId, BlockTable};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::chunk::store::{give_scratch, take_scratch};
use crate::shared::math::compare::{not_greater, not_less, not_less_or_equal};
use crate::shared::rng::random::RandomSource;
use crate::shared::rng::xoroshiro::PositionalFactory;
use crate::shared::structure::Adjustments;
use crate::shared::terrain::{CELL_HEIGHT, CELL_WIDTH, Corner, NoiseChunk};
use chorus_block::block_id::{COPPER_ORE, DEEPSLATE, DEEPSLATE_IRON_ORE, GRANITE, RAW_COPPER_BLOCK, RAW_IRON_BLOCK, TUFF};

const WIDTH_RECIPROCAL: f32 = 0.25;
const HEIGHT_RECIPROCAL: f32 = 0.125;
const SQUEEZE_SCALE: f32 = 0.005;
const SQUEEZE_DIVISOR: f32 = -24.0;
const SQUEEZE_HALF: f32 = 0.5;
const NOODLE_FLOOR_OFFSET: i32 = 4;
const NOODLE_RIDGE_SCALE: f32 = 1.5;
const NOODLE_THICKNESS_NARROW: f32 = 0.05;
const NOODLE_THICKNESS_WIDE: f32 = 0.1;
const NOODLE_THICKNESS_SPAN: f32 = 0.05;
const DEEPSLATE_BLEND_TOP: i32 = 8;
const DEEPSLATE_BLEND_STEP: f32 = 0.125;
const COPPER_VEIN: VeinRange = VeinRange { min_y: 0, max_y: 50 };
const IRON_VEIN: VeinRange = VeinRange { min_y: -60, max_y: -8 };
const VEIN_EDGE_SPAN: f32 = 20.0;
const VEIN_EDGE_SLOPE: f32 = 0.2;
const VEIN_EDGE_OUTSIDE: f32 = -0.2;
const VEIN_MINIMUM: f32 = 0.4;
const VEIN_CHANCE: f32 = 0.7;
const VEIN_RIDGE_WIDTH: f32 = -0.08;
const RICHNESS_SPAN: f32 = 0.20000002;
const RICHNESS_MIN: f32 = 0.1;
const RICHNESS_MAX: f32 = 0.3;
const GAP_FLOOR: f32 = -1.0;
const GAP_CEILING: f32 = 1.0;
const GAP_THRESHOLD: f32 = -0.3;
const RAW_BLOCK_CHANCE: f32 = 0.02;
const DENSITY_MARGIN: f32 = 0.01;
const NOODLE_MARGIN: f32 = 0.01;
const VEIN_SAFE_STRENGTH: f32 = 0.39;
const NOODLE_RIDGE_SAFE: f32 = 0.07;
const VEIN_RIDGE_SAFE: f32 = 0.085;
const CELLS: i32 = 16 / CELL_WIDTH;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CellKind {
    Air,
    Solid,
    Mixed,
}

#[derive(Clone, Copy)]
pub struct Fluids {
    pub water: BlockId,
    pub lava: BlockId,
}

struct VeinRange {
    min_y: i32,
    max_y: i32,
}

struct Vein {
    range: VeinRange,
    ore: BlockId,
    raw: BlockId,
    filler: BlockId,
}

pub struct TerrainFill {
    stone: BlockId,
    deepslate: BlockId,
    copper: Vein,
    iron: Vein,
    depth: PositionalFactory,
    ore: PositionalFactory,
    ore_gap: NormalNoise,
    pub aquifer: AquiferNoises,
}

struct Cell<'a>([[[&'a Corner; 2]; 2]; 2]);

#[derive(Clone, Copy)]
struct Lane {
    low: f32,
    high: f32,
}

impl Lane {
    fn at(self, fy: f32) -> f32 {
        self.low + fy * (self.high - self.low)
    }
}

impl Cell<'_> {
    fn lane(&self, field: fn(&Corner) -> f32, fx: f32, fz: f32) -> Lane {
        let along_z = |a: usize, b: usize| {
            let near = field(self.0[a][b][0]);
            let far = field(self.0[a][b][1]);
            near + fz * (far - near)
        };
        let (v00, v01, v10, v11) = (along_z(0, 0), along_z(0, 1), along_z(1, 0), along_z(1, 1));
        Lane {
            low: v00 + fx * (v10 - v00),
            high: v01 + fx * (v11 - v01),
        }
    }
}

#[derive(Default)]
struct Lanes {
    toggle: Option<Lane>,
    thickness: Option<Lane>,
    ridge_a: Option<Lane>,
    ridge_b: Option<Lane>,
    vein_toggle: Option<Lane>,
    vein_a: Option<Lane>,
    vein_b: Option<Lane>,
}

fn lazy(slot: &mut Option<Lane>, cell: &Cell, field: fn(&Corner) -> f32, fx: f32, fz: f32) -> Lane {
    *slot.get_or_insert_with(|| cell.lane(field, fx, fz))
}

const VEIN_MIN_Y: i32 = -60;
const VEIN_MAX_Y: i32 = 50;

struct Range {
    low: f32,
    high: f32,
}

impl Range {
    const EMPTY: Self = Self {
        low: f32::INFINITY,
        high: f32::NEG_INFINITY,
    };

    fn add(&mut self, value: f32) {
        self.low = self.low.min(value);
        self.high = self.high.max(value);
    }

    fn keeps_clear_of_zero(&self, distance: f32) -> bool {
        self.low > distance || self.high < -distance
    }
}

fn classify(noise: &NoiseChunk, cx: i32, cy: i32, cz: i32, y_low: i32, veins: bool) -> CellKind {
    let (mut main, mut noodle, mut ridge_a, mut ridge_b) = (Range::EMPTY, Range::EMPTY, Range::EMPTY, Range::EMPTY);
    let (mut vein_strength, mut vein_a, mut vein_b) = (0.0f32, Range::EMPTY, Range::EMPTY);
    for a in 0..2 {
        for b in 0..2 {
            for c in 0..2 {
                let corner = noise.corner(cx + a, cy + b, cz + c);
                main.add(corner.main);
                noodle.add(corner.noodle);
                ridge_a.add(corner.noodle_ridge_a);
                ridge_b.add(corner.noodle_ridge_b);
                vein_strength = vein_strength.max(corner.vein_toggle.abs());
                vein_a.add(corner.vein_a);
                vein_b.add(corner.vein_b);
            }
        }
    }
    if main.high < -DENSITY_MARGIN {
        return CellKind::Air;
    }
    if not_greater(main.low, DENSITY_MARGIN) {
        return CellKind::Mixed;
    }
    let noodle_absent = noodle.high < -NOODLE_MARGIN || ridge_a.keeps_clear_of_zero(NOODLE_RIDGE_SAFE) || ridge_b.keeps_clear_of_zero(NOODLE_RIDGE_SAFE);
    if !noodle_absent {
        return CellKind::Mixed;
    }
    let y_high = y_low + CELL_HEIGHT - 1;
    let vein_absent = vein_strength < VEIN_SAFE_STRENGTH || vein_a.keeps_clear_of_zero(VEIN_RIDGE_SAFE) || vein_b.keeps_clear_of_zero(VEIN_RIDGE_SAFE);
    if veins && y_high >= VEIN_MIN_Y && y_low <= VEIN_MAX_Y && !vein_absent {
        return CellKind::Mixed;
    }
    CellKind::Solid
}

fn squeeze(density: f32) -> f32 {
    let t = (density * SQUEEZE_SCALE).clamp(-1.0, 1.0);
    t * t * t / SQUEEZE_DIVISOR + t * SQUEEZE_HALF
}

impl TerrainFill {
    pub fn new(root: &PositionalFactory, table: &mut BlockTable, stone: BlockId) -> Self {
        let mut block = |name: &'static str| table.name(name);
        Self {
            stone,
            deepslate: block(DEEPSLATE),
            copper: Vein {
                range: COPPER_VEIN,
                ore: block(COPPER_ORE),
                raw: block(RAW_COPPER_BLOCK),
                filler: block(GRANITE),
            },
            iron: Vein {
                range: IRON_VEIN,
                ore: block(DEEPSLATE_IRON_ORE),
                raw: block(RAW_IRON_BLOCK),
                filler: block(TUFF),
            },
            depth: root.hash_of("minecraft:depth_based_layer").fork_positional(),
            ore: root.hash_of("minecraft:ore").fork_positional(),
            ore_gap: NormalNoise::new(&mut root.hash_of("minecraft:ore_gap"), -5, &[1.0]),
            aquifer: AquiferNoises::new(root),
        }
    }

    fn default_block(&self, x: i32, y: i32, z: i32) -> BlockId {
        if y < 0 {
            return self.deepslate;
        }
        if y > DEEPSLATE_BLEND_TOP {
            return self.stone;
        }
        let height = y as f32 * DEEPSLATE_BLEND_STEP;
        let chance = if height > 1.0 { 0.0 } else { 1.0 - height };
        if chance > self.depth.at(x, y, z).next_float() { self.deepslate } else { self.stone }
    }

    fn vein_block(&self, veininess: f32, ridges: impl FnOnce() -> (f32, f32), x: i32, y: i32, z: i32, default: BlockId) -> BlockId {
        let vein = if veininess > 0.0 { &self.copper } else { &self.iron };
        let above = y as f32 - vein.range.min_y as f32;
        if not_less_or_equal(0.0, above) {
            return default;
        }
        let below = vein.range.max_y as f32 - y as f32;
        if not_less_or_equal(0.0, below) {
            return default;
        }
        let edge_distance = above.min(below) / VEIN_EDGE_SPAN;
        let edge = if 0.0 <= edge_distance {
            if edge_distance <= 1.0 { edge_distance * VEIN_EDGE_SLOPE + VEIN_EDGE_OUTSIDE } else { 0.0 }
        } else {
            VEIN_EDGE_OUTSIDE
        };
        let strength = veininess.abs();
        if not_less_or_equal(VEIN_MINIMUM, edge + strength) {
            return default;
        }
        let mut random = self.ore.at(x, y, z);
        if not_less_or_equal(random.next_float(), VEIN_CHANCE) {
            return default;
        }
        let (ridge_a, ridge_b) = ridges();
        let ridge = (ridge_a.abs() + VEIN_RIDGE_WIDTH).max(ridge_b.abs() + VEIN_RIDGE_WIDTH);
        if not_less(ridge, 0.0) {
            return default;
        }
        let progress = (strength - VEIN_MINIMUM) / RICHNESS_SPAN;
        let richness = if 0.0 <= progress {
            if progress <= 1.0 { progress * RICHNESS_SPAN + RICHNESS_MIN } else { RICHNESS_MAX }
        } else {
            RICHNESS_MIN
        };
        let gap = self.ore_gap.value(x as f32, y as f32, z as f32);
        if random.next_float() < richness && GAP_FLOOR <= gap && gap.min(GAP_CEILING) > GAP_THRESHOLD {
            return if random.next_float() < RAW_BLOCK_CHANCE { vein.raw } else { vein.ore };
        }
        vein.filler
    }

    pub fn run(&self, terrain: &BedrockTerrain, noise: &NoiseChunk, adjustments: &Adjustments, chunk: &mut ProtoChunk, window: SurfaceWindow, fluids: Fluids) -> Aquifer {
        let (chunk_x, chunk_z) = (chunk.x, chunk.z);
        let veins = terrain.ore_veins_enabled();
        let (min_y, noise_cells) = (terrain.min_y(), terrain.noise_levels() as i32 - 1);
        let mut sampler = AquiferSampler::new(&self.aquifer, terrain, window.clone(), chunk_x, chunk_z);
        let dry_above = sampler.dry_above();
        let kinds: Vec<CellKind> = (0..CELLS * CELLS)
            .flat_map(|index| (0..noise_cells).map(move |cy| (index, cy)))
            .map(|(index, cy)| classify(noise, index % CELLS, cy, index / CELLS, min_y + cy * CELL_HEIGHT, veins))
            .collect();
        let mut result = Aquifer { scheduled_ticks: Vec::new(), window };
        let mut chunk = chunk.edit();
        let mut blocks = take_scratch(chunk.height as usize * 256, AIR);
        let mut tops = [min_y as i16; 256];
        let mut put = |lx: i32, y: i32, lz: i32, block: BlockId| {
            let column = ((lz << 4) | lx) as usize;
            blocks[(y - min_y) as usize * 256 + column] = block;
            if block != AIR {
                tops[column] = tops[column].max(y as i16 + 1);
            }
        };
        for lz in 0..16 {
            for lx in 0..16 {
                let (cx, cz) = (lx / CELL_WIDTH, lz / CELL_WIDTH);
                let fx = (lx % CELL_WIDTH) as f32 * WIDTH_RECIPROCAL;
                let fz = (lz % CELL_WIDTH) as f32 * WIDTH_RECIPROCAL;
                let (x, z) = (chunk.min_x() + lx, chunk.min_z() + lz);
                let column = adjustments.column(lx, lz);
                let adjusted = !column.is_empty();
                for cy in 0..noise_cells {
                    if !adjusted {
                        let y_low = min_y + cy * CELL_HEIGHT;
                        match kinds[((cz * CELLS + cx) * noise_cells + cy) as usize] {
                            CellKind::Air if y_low >= dry_above => continue,
                            CellKind::Solid => {
                                for ly in 0..CELL_HEIGHT {
                                    let y = y_low + ly;
                                    put(lx, y, lz, self.default_block(x, y, z));
                                }
                                continue;
                            }
                            _ => {}
                        }
                    }
                    let cell = Cell(std::array::from_fn(|a| {
                        std::array::from_fn(|b| std::array::from_fn(|c| noise.corner(cx + a as i32, cy + b as i32, cz + c as i32)))
                    }));
                    let main = cell.lane(|corner| corner.main, fx, fz);
                    let mut lanes = Lanes::default();
                    for ly in 0..CELL_HEIGHT {
                        let y = min_y + cy * CELL_HEIGHT + ly;
                        let fy = ly as f32 * HEIGHT_RECIPROCAL;
                        let mut value = squeeze(main.at(fy));
                        if y >= min_y + NOODLE_FLOOR_OFFSET && value > 0.0 {
                            let toggle = lazy(&mut lanes.toggle, &cell, |corner| corner.noodle, fx, fz).at(fy);
                            if not_greater(0.0, toggle) {
                                let thickness = (lazy(&mut lanes.thickness, &cell, |corner| corner.noodle_thickness, fx, fz).at(fy) + 1.0) * 0.5;
                                let width = if thickness < 0.0 {
                                    NOODLE_THICKNESS_NARROW
                                } else if thickness > 1.0 {
                                    NOODLE_THICKNESS_WIDE
                                } else {
                                    thickness * NOODLE_THICKNESS_SPAN + NOODLE_THICKNESS_NARROW
                                };
                                let ridge_a = (lazy(&mut lanes.ridge_a, &cell, |corner| corner.noodle_ridge_a, fx, fz).at(fy) * NOODLE_RIDGE_SCALE).abs() - width;
                                let ridge_b = (lazy(&mut lanes.ridge_b, &cell, |corner| corner.noodle_ridge_b, fx, fz).at(fy) * NOODLE_RIDGE_SCALE).abs() - width;
                                value = value.min(if ridge_b > ridge_a { ridge_b } else { ridge_a });
                            }
                        }
                        if !column.is_empty() {
                            value = column.apply(x, y, z, value);
                        }
                        let mut solid = value > 0.0;
                        let mut substance = None;
                        if !solid {
                            let found = sampler.substance(x, y, z);
                            solid = value + found.barrier > 0.0;
                            substance = Some(found);
                        }
                        if solid {
                            let default = self.default_block(x, y, z);
                            let block = if veins && (VEIN_MIN_Y..=VEIN_MAX_Y).contains(&y) {
                                let veininess = lazy(&mut lanes.vein_toggle, &cell, |corner| corner.vein_toggle, fx, fz).at(fy);
                                let (vein_a, vein_b) = (&mut lanes.vein_a, &mut lanes.vein_b);
                                self.vein_block(
                                    veininess,
                                    || (lazy(vein_a, &cell, |corner| corner.vein_a, fx, fz).at(fy), lazy(vein_b, &cell, |corner| corner.vein_b, fx, fz).at(fy)),
                                    x,
                                    y,
                                    z,
                                    default,
                                )
                            } else {
                                default
                            };
                            put(lx, y, lz, block);
                        } else if let Some(found) = substance.filter(|found| y < found.level) {
                            put(lx, y, lz, if found.lava { fluids.lava } else { fluids.water });
                            if found.schedule_tick {
                                result.scheduled_ticks.push(FluidTick {
                                    x: lx as u8,
                                    z: lz as u8,
                                    y: y as i16,
                                    lava: found.lava,
                                });
                            }
                        }
                    }
                }
            }
        }
        chunk.fill_empty(&blocks);
        chunk.set_tops(&tops);
        give_scratch(blocks);
        result
    }
}
