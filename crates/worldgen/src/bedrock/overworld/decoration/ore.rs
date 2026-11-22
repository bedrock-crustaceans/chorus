use super::sine;
use crate::shared::blocks::{self, BlockId, BlockTable, State};
use crate::shared::chunk::region::{BlockView, Region};
use crate::shared::rng::mersenne::MersenneTwister;
use atomicow::CowArc;
use chorus_block::block_id::{ANDESITE, DIORITE, GRANITE, POLISHED_ANDESITE, POLISHED_DIORITE, POLISHED_GRANITE, STONE};
use glam::IVec3;
use serde_json::Value;

pub const MAX_STEPS: usize = 64;
const HALF_TURN: f32 = std::f32::consts::PI;
const SPREAD_DIVISOR: f32 = 8.0;
const RADIUS_DIVISOR: f32 = 16.0;
const CELL_CENTRE: f32 = 0.5;
const CHUNK_CENTRE: f32 = 8.0;
const ENDPOINT_Y_BOUND: i32 = 3;
const ENDPOINT_Y_SHIFT: f32 = 2.0;
const DEGENERATE_LENGTH: f32 = 0.0001;
const FULL_DISCARD: f32 = 1.0;
const NEIGHBOURS: [IVec3; 6] = [IVec3::NEG_Y, IVec3::Y, IVec3::NEG_Z, IVec3::Z, IVec3::NEG_X, IVec3::X];
const STONE_TYPE: &str = "stone_type";
const STONE_VARIANTS: [(&str, &str); 6] = [
    ("andesite", ANDESITE),
    ("diorite", DIORITE),
    ("granite", GRANITE),
    ("andesite_smooth", POLISHED_ANDESITE),
    ("diorite_smooth", POLISHED_DIORITE),
    ("granite_smooth", POLISHED_GRANITE),
];

pub trait Draw {
    fn float(&mut self) -> f32;
    fn below(&mut self, bound: i32) -> i32;
    fn word(&mut self) -> u32;
    fn double(&mut self) -> f64;
    fn boolean(&mut self) -> bool;
}

impl Draw for MersenneTwister {
    fn float(&mut self) -> f32 {
        self.next_float()
    }

    fn below(&mut self, bound: i32) -> i32 {
        self.next_int_bounded(bound)
    }

    fn word(&mut self) -> u32 {
        self.next_u32()
    }

    fn double(&mut self) -> f64 {
        self.next_double()
    }

    fn boolean(&mut self) -> bool {
        self.next_boolean()
    }
}

pub trait Cells {
    fn block(&self, pos: IVec3) -> BlockId;
    fn place(&mut self, pos: IVec3, block: BlockId);
    fn place_replaceable(&mut self, pos: IVec3, block: BlockId, _may_replace: &[u16]) {
        self.place(pos, block);
    }
    fn kind(&self, block: BlockId) -> u16;
    fn is_air(&self, block: BlockId) -> bool;
    fn min_y(&self) -> i32;
    fn max_y(&self) -> i32;
}

pub struct RegionCells<'a, 'b>(pub &'a mut Region<'b>);

impl Cells for RegionCells<'_, '_> {
    fn block(&self, pos: IVec3) -> BlockId {
        self.0.get(pos)
    }

    fn place(&mut self, pos: IVec3, block: BlockId) {
        self.0.set(pos, block);
    }

    fn place_replaceable(&mut self, pos: IVec3, block: BlockId, may_replace: &[u16]) {
        self.0.replace(pos, block, may_replace);
    }

    fn kind(&self, block: BlockId) -> u16 {
        self.0.blocks().entry(block).kind
    }

    fn is_air(&self, block: BlockId) -> bool {
        self.0.blocks().is_air(block)
    }

    fn min_y(&self) -> i32 {
        self.0.min_y()
    }

    fn max_y(&self) -> i32 {
        self.0.max_y()
    }
}

#[derive(Debug)]
pub struct ReplaceRule {
    pub block: BlockId,
    pub may_replace: Vec<u16>,
}

#[derive(Debug)]
pub struct Ore {
    pub(super) count: i32,
    pub(super) discard_chance: f32,
    pub(super) rules: Vec<ReplaceRule>,
}

#[derive(Clone, Copy, Default)]
struct Sphere {
    centre: [f32; 3],
    radius_squared: f32,
    radius: f32,
}

pub(super) fn block_state(key: &'static str, value: &'static Value) -> Option<State> {
    match value {
        Value::Bool(flag) => Some((CowArc::Static(key), blocks::flag(*flag))),
        Value::Number(number) => Some((CowArc::Static(key), blocks::int(number.as_i64()? as i32))),
        Value::String(text) => Some((CowArc::Static(key), blocks::text(text))),
        _ => None,
    }
}

pub fn resolve_block(table: &mut BlockTable, descriptor: &'static Value) -> Option<BlockId> {
    let (name, states) = match descriptor {
        Value::String(name) => (name.as_str(), None),
        Value::Object(map) => (map.get("name")?.as_str()?, map.get("states").and_then(Value::as_object)),
        _ => return None,
    };
    if name == STONE {
        let variant = states.and_then(|states| states.get(STONE_TYPE)).and_then(Value::as_str);
        if let Some((_, modern)) = STONE_VARIANTS.iter().find(|(legacy, _)| Some(*legacy) == variant) {
            return Some(table.name(*modern));
        }
        return Some(table.name(STONE));
    }
    let states: Vec<State> = states.into_iter().flatten().filter_map(|(key, value)| block_state(key, value)).collect();
    Some(table.get(name, &states))
}

impl Ore {
    pub fn custom(count: i32, rules: Vec<ReplaceRule>) -> Self {
        Self { count, discard_chance: 0.0, rules }
    }
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let count = body.get("count")?.as_i64()? as i32;
        if count < 1 || count as usize > MAX_STEPS {
            return None;
        }
        let rules = body["replace_rules"]
            .as_array()?
            .iter()
            .map(|rule| {
                let block = resolve_block(table, rule.get("places_block")?)?;
                let may_replace = rule["may_replace"]
                    .as_array()?
                    .iter()
                    .map(|descriptor| Some(table.kind(descriptor.get("name")?.as_str()?)))
                    .collect::<Option<Vec<_>>>()?;
                Some(ReplaceRule { block, may_replace })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            count,
            discard_chance: body.get("discard_chance_on_air_exposure").and_then(Value::as_f64).unwrap_or(0.0) as f32,
            rules,
        })
    }

    #[cfg(test)]
    pub fn place_blocks(&self) -> impl Iterator<Item = BlockId> + '_ {
        self.rules.iter().map(|rule| rule.block)
    }

    pub fn place<W: Cells, D: Draw>(&self, world: &mut W, random: &mut D, origin: IVec3) -> bool {
        let count = self.count as usize;
        let count_float = self.count as f32;
        let inverse = 1.0 / count_float;
        let spread = count_float / SPREAD_DIVISOR;

        let angle = random.float() * HALF_TURN;
        let (axis_x, axis_z) = (sine::sin(angle) * spread, sine::cos(angle) * spread);
        let [origin_x, origin_y, origin_z] = [origin.x as f32, origin.y as f32, origin.z as f32];
        let x1 = origin_x + (CHUNK_CENTRE + axis_x);
        let z1 = origin_z + (CHUNK_CENTRE + axis_z);
        let y1 = origin_y + (random.below(ENDPOINT_Y_BOUND) as f32 - ENDPOINT_Y_SHIFT);
        let x2 = origin_x + (CHUNK_CENTRE - axis_x);
        let z2 = origin_z + (CHUNK_CENTRE - axis_z);
        let y2 = origin_y + (random.below(ENDPOINT_Y_BOUND) as f32 - ENDPOINT_Y_SHIFT);
        if self.rules.is_empty() {
            return false;
        }

        let (dx, dy, dz) = (x2 - x1, y2 - y1, z2 - z1);
        let axis = Axis::new([x1, y1, z1], [dx, dy, dz]);
        let step = HALF_TURN * inverse;
        let mut spheres = [Sphere::default(); MAX_STEPS];
        for (index, sphere) in spheres[..count].iter_mut().enumerate() {
            let fraction = index as f32;
            let scale = random.float() * count_float / RADIUS_DIVISOR;
            let radius = ((sine::sin(fraction * step) + 1.0) * scale + 1.0) / 2.0;
            *sphere = Sphere {
                centre: [x1 + dx * fraction * inverse, y1 + dy * fraction * inverse, z1 + dz * fraction * inverse],
                radius_squared: radius * radius,
                radius: (radius * radius).sqrt(),
            };
        }

        let mut kept = [true; MAX_STEPS];
        for (index, sphere) in spheres[..count].iter().enumerate() {
            kept[index] = !spheres[index + 1..count]
                .iter()
                .any(|later| sphere.radius + distance_squared(later.centre, sphere.centre).sqrt() < later.radius);
        }
        self.fill(world, random, &spheres[..count], &kept[..count], &axis)
    }

    fn fill<W: Cells, D: Draw>(&self, world: &mut W, random: &mut D, spheres: &[Sphere], kept: &[bool], axis: &Axis) -> bool {
        let (mut low, mut high) = ([i32::MAX; 3], [i32::MIN; 3]);
        let mut widest = 0.0f32;
        for sphere in spheres.iter().zip(kept).filter(|(_, kept)| **kept).map(|(sphere, _)| sphere) {
            for (axis, centre) in sphere.centre.into_iter().enumerate() {
                low[axis] = low[axis].min((centre - sphere.radius).floor() as i32);
                high[axis] = high[axis].max((centre + sphere.radius).floor() as i32);
            }
            if sphere.radius_squared > widest {
                widest = sphere.radius_squared;
            }
        }
        let mut near: Vec<&Sphere> = Vec::with_capacity(spheres.len());
        let mut placed = false;
        for x in low[0]..=high[0] {
            for z in low[2]..=high[2] {
                let (cell_x, cell_z) = (x as f32 + CELL_CENTRE, z as f32 + CELL_CENTRE);
                if !axis.column_near(cell_x, cell_z, widest) {
                    continue;
                }
                near.clear();
                near.extend(
                    spheres
                        .iter()
                        .zip(kept)
                        .filter(|(sphere, kept)| **kept && axis_reach(cell_x, sphere.centre[0]) < sphere.radius_squared && axis_reach(cell_z, sphere.centre[2]) < sphere.radius_squared)
                        .map(|(sphere, _)| sphere),
                );
                if near.is_empty() {
                    continue;
                }
                for y in low[1].max(world.min_y())..=high[1].min(world.max_y()) {
                    let cell = [cell_x, y as f32 + CELL_CENTRE, cell_z];
                    if axis.cell_near(cell, widest) && near.iter().any(|sphere| distance_squared(cell, sphere.centre) < sphere.radius_squared) {
                        placed |= self.replace(world, random, IVec3::new(x, y, z));
                    }
                }
            }
        }
        placed
    }

    fn replace<W: Cells, D: Draw>(&self, world: &mut W, random: &mut D, pos: IVec3) -> bool {
        let target = world.block(pos);
        let kind = world.kind(target);
        for rule in &self.rules {
            if !rule.may_replace.contains(&kind) || target == rule.block {
                continue;
            }
            if self.discard_chance > 0.0 && self.discarded(world, random, pos) {
                continue;
            }
            world.place_replaceable(pos, rule.block, &rule.may_replace);
            return true;
        }
        false
    }

    fn discarded<W: Cells, D: Draw>(&self, world: &W, random: &mut D, pos: IVec3) -> bool {
        let candidate = self.discard_chance >= FULL_DISCARD || random.float() < self.discard_chance;
        candidate && NEIGHBOURS.iter().any(|&offset| world.is_air(world.block(pos + offset)))
    }
}

fn axis_reach(cell: f32, centre: f32) -> f32 {
    (cell - centre) * (cell - centre)
}

fn distance_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (dx, dy, dz) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    dx * dx + dy * dy + dz * dz
}

struct Axis {
    start: [f32; 3],
    unit: [f32; 3],
    flat: [f32; 2],
}

impl Axis {
    fn new(start: [f32; 3], [dx, dy, dz]: [f32; 3]) -> Self {
        let length = (dz * dz + dy * dy + dx * dx).sqrt();
        let unit = if length < DEGENERATE_LENGTH { [0.0; 3] } else { [dx / length, dy / length, dz / length] };
        let flat_length = (unit[2] * unit[2] + unit[0] * unit[0]).sqrt();
        let flat = if flat_length < DEGENERATE_LENGTH {
            [0.0; 2]
        } else {
            [unit[0] / flat_length, unit[2] / flat_length]
        };
        Self { start, unit, flat }
    }

    fn column_near(&self, cell_x: f32, cell_z: f32, widest: f32) -> bool {
        let [start_x, _, start_z] = self.start;
        let [flat_x, flat_z] = self.flat;
        let along = flat_z * (cell_z - start_z) + flat_x * (cell_x - start_x);
        let across_x = cell_x - (flat_x * along + start_x);
        let across_z = cell_z - (along * flat_z + start_z);
        across_z * across_z + across_x * across_x <= widest
    }

    fn cell_near(&self, [cell_x, cell_y, cell_z]: [f32; 3], widest: f32) -> bool {
        let [start_x, start_y, start_z] = self.start;
        let [unit_x, unit_y, unit_z] = self.unit;
        let along = (cell_y - start_y) * unit_y + (cell_x - start_x) * unit_x + (cell_z - start_z) * unit_z;
        let across_x = cell_x - (unit_x * along + start_x);
        let across_y = cell_y - (unit_y * along + start_y);
        let across_z = cell_z - (along * unit_z + start_z);
        across_z * across_z + across_y * across_y + across_x * across_x <= widest
    }
}
