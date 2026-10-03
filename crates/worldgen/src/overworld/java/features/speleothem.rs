use glam::IVec3;

use super::super::blocks::{BlockId, BlockTable, flag, state, text};
use super::super::carver::{cos, sin};
use super::super::random::RandomSource;
use super::super::tags::Tag;
use super::Context;
use super::predicate::{Direction, Fluid, fluid_at};
use super::provider::{FloatProvider, IntProvider};
use super::region::{Heightmap, Region};
use chorus_block::block_id::{LAVA, WATER};
use chorus_block::state::common::{DRIPSTONE_THICKNESS, HANGING};

const THICKNESSES: [&str; 5] = ["merge", "tip", "frustum", "middle", "base"];

#[derive(Clone, Copy)]
enum Thickness {
    TipMerge,
    Tip,
    Frustum,
    Middle,
    Base,
}

pub struct SpeleothemBlocks {
    pub base: BlockId,
    pointed_kind: u16,
    pointed: Vec<BlockId>,
    pub replaceable: Tag,
    lava: u16,
    water: u16,
    water_block: BlockId,
}

impl SpeleothemBlocks {
    pub fn new(table: &mut BlockTable, base: &'static str, pointed: &'static str, replaceable: Tag) -> Self {
        let mut states = Vec::with_capacity(20);
        for hanging in [true, false] {
            for thickness in THICKNESSES {
                let dry = table.get(pointed, &[state(DRIPSTONE_THICKNESS, text(thickness)), state(HANGING, flag(hanging))]);
                states.push(dry);
                states.push(table.flooded(dry));
            }
        }
        Self {
            base: table.name(base),
            pointed_kind: table.kind(pointed),
            pointed: states,
            replaceable,
            lava: table.kind(LAVA),
            water: table.kind(WATER),
            water_block: table.name(WATER),
        }
    }

    fn pointed(&self, up: bool, thickness: Thickness, waterlogged: bool) -> BlockId {
        self.pointed[(up as usize * 5 + thickness as usize) * 2 + waterlogged as usize]
    }

    fn kind(&self, region: &Region, pos: IVec3) -> u16 {
        region.blocks.entry(region.get(pos)).kind
    }

    fn is_base(&self, region: &Region, state: BlockId) -> bool {
        region.blocks.same_kind(state, self.base) || region.blocks.is(state, self.replaceable)
    }

    fn is_empty_or_water(&self, region: &Region, pos: IVec3) -> bool {
        let state = region.get(pos);
        region.blocks.is_air(state) || region.blocks.entry(state).kind == self.water
    }

    fn is_empty_or_water_or_lava(&self, region: &Region, pos: IVec3) -> bool {
        self.is_empty_or_water(region, pos) || self.kind(region, pos) == self.lava
    }

    fn place_base_if_possible(&self, region: &mut Region, pos: IVec3) -> bool {
        if region.blocks.is(region.get(pos), self.replaceable) {
            region.set(pos, self.base);
            true
        } else {
            false
        }
    }

    fn grow(&self, region: &mut Region, start: IVec3, tip: Direction, height: i32, merged: bool) {
        let up = tip == Direction::Up;
        if !self.is_base(region, region.get(start - tip.offset())) {
            return;
        }
        let mut column = Vec::new();
        if height >= 3 {
            column.push(Thickness::Base);
            for _ in 0..height - 3 {
                column.push(Thickness::Middle);
            }
        }
        if height >= 2 {
            column.push(Thickness::Frustum);
        }
        if height >= 1 {
            column.push(if merged { Thickness::TipMerge } else { Thickness::Tip });
        }
        let mut pos = start;
        for thickness in column {
            let wet = fluid_at(region.blocks, region.get(pos)) == Some(Fluid::Water);
            region.set(pos, self.pointed(up, thickness, wet));
            pos += tip.offset();
        }
    }
}

enum Column {
    Line,
    Range { floor: i32, ceiling: i32 },
    Ray { edge: i32, up: bool },
}

impl Column {
    fn create(floor: Option<i32>, ceiling: Option<i32>) -> Self {
        match (floor, ceiling) {
            (Some(floor), Some(ceiling)) => Self::Range { floor, ceiling },
            (Some(edge), None) => Self::Ray { edge, up: true },
            (None, Some(edge)) => Self::Ray { edge, up: false },
            (None, None) => Self::Line,
        }
    }

    fn ceiling(&self) -> Option<i32> {
        match *self {
            Self::Range { ceiling, .. } => Some(ceiling),
            Self::Ray { edge, up: false } => Some(edge),
            _ => None,
        }
    }

    fn floor(&self) -> Option<i32> {
        match *self {
            Self::Range { floor, .. } => Some(floor),
            Self::Ray { edge, up: true } => Some(edge),
            _ => None,
        }
    }

    fn height(&self) -> Option<i32> {
        match *self {
            Self::Range { floor, ceiling } => Some(ceiling - floor - 1),
            _ => None,
        }
    }

    fn scan(region: &Region, pos: IVec3, range: i32, inside: impl Fn(&Region, IVec3) -> bool, edge: impl Fn(&Region, IVec3) -> bool) -> Option<Self> {
        if !inside(region, pos) {
            return None;
        }
        let scan = |step: i32| {
            let mut cursor = pos;
            let mut i = 1;
            while i < range && inside(region, cursor) {
                cursor.y += step;
                i += 1;
            }
            edge(region, cursor).then_some(cursor.y)
        };
        let ceiling = scan(1);
        let floor = scan(-1);
        Some(Self::create(floor, ceiling))
    }
}

pub struct Speleothem {
    pub blocks: SpeleothemBlocks,
    pub taller_chance: f32,
    pub directional_spread: f32,
    pub spread_radius2: f32,
    pub spread_radius3: f32,
}

fn random_direction(random: &mut dyn RandomSource) -> Direction {
    Direction::ALL[random.next_int_bounded(6) as usize]
}

impl Speleothem {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let region = &mut *ctx.region;
        let above = self.blocks.is_base(region, region.get(origin + IVec3::Y));
        let below = self.blocks.is_base(region, region.get(origin - IVec3::Y));
        let tip = match (above, below) {
            (true, true) => {
                if random.next_boolean() {
                    Direction::Down
                } else {
                    Direction::Up
                }
            }
            (true, false) => Direction::Down,
            (false, true) => Direction::Up,
            (false, false) => return false,
        };
        let root = origin - tip.offset();
        self.blocks.place_base_if_possible(region, root);
        for direction in Direction::HORIZONTAL {
            if random.next_float() > self.directional_spread {
                continue;
            }
            let first = root + direction.offset();
            self.blocks.place_base_if_possible(region, first);
            if random.next_float() > self.spread_radius2 {
                continue;
            }
            let second = first + random_direction(random).offset();
            self.blocks.place_base_if_possible(region, second);
            if random.next_float() <= self.spread_radius3 {
                let third = second + random_direction(random).offset();
                self.blocks.place_base_if_possible(region, third);
            }
        }
        let height = if random.next_float() < self.taller_chance && self.blocks.is_empty_or_water(region, origin + tip.offset()) {
            2
        } else {
            1
        };
        self.blocks.grow(region, origin, tip, height, false);
        true
    }
}

pub struct SpeleothemCluster {
    pub blocks: SpeleothemBlocks,
    pub search_range: i32,
    pub height: IntProvider,
    pub radius: IntProvider,
    pub max_height_diff: i32,
    pub height_deviation: i32,
    pub layer_thickness: IntProvider,
    pub density: FloatProvider,
    pub wetness: FloatProvider,
    pub chance_at_max_distance: f32,
    pub max_edge_distance: i32,
    pub max_center_distance: i32,
}

fn clamped_map(value: f64, from_min: f64, from_max: f64, to_min: f64, to_max: f64) -> f64 {
    let t = ((value - from_min) / (from_max - from_min)).clamp(0.0, 1.0);
    to_min + t * (to_max - to_min)
}

fn between_inclusive(random: &mut dyn RandomSource, min: i32, max: i32) -> i32 {
    random.next_int_bounded(max - min + 1) + min
}

impl SpeleothemCluster {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let region = &mut *ctx.region;
        if !self.blocks.is_empty_or_water(region, origin) {
            return false;
        }
        let height = self.height.sample(random);
        let wetness = self.wetness.sample(random);
        let density = self.density.sample(random);
        let x_radius = self.radius.sample(random);
        let z_radius = self.radius.sample(random);
        for dx in -x_radius..=x_radius {
            for dz in -z_radius..=z_radius {
                let edge = (x_radius - dx.abs()).min(z_radius - dz.abs());
                let t = (edge as f32 / self.max_edge_distance as f32).clamp(0.0, 1.0);
                let chance = (self.chance_at_max_distance + t * (1.0 - self.chance_at_max_distance)) as f64;
                self.place_column(region, random, origin + IVec3::new(dx, 0, dz), dx, dz, wetness, chance, height, density);
            }
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn place_column(&self, region: &mut Region, random: &mut dyn RandomSource, pos: IVec3, dx: i32, dz: i32, wetness: f32, chance: f64, cluster_height: i32, density: f32) {
        let blocks = &self.blocks;
        let Some(base) = Column::scan(region, pos, self.search_range, |r, p| blocks.is_empty_or_water(r, p), |r, p| !blocks.is_empty_or_water(r, p)) else {
            return;
        };
        let ceiling = base.ceiling();
        let base_floor = base.floor();
        if ceiling.is_none() && base_floor.is_none() {
            return;
        }
        let at = |y: i32| IVec3::new(pos.x, y, pos.z);
        let want_pool = random.next_float() < wetness;
        let column = match base_floor {
            Some(floor) if want_pool && self.can_place_pool(region, at(floor)) => {
                region.set(at(floor), blocks.water_block);
                Column::create(Some(floor - 1), ceiling)
            }
            _ => base,
        };
        let floor = column.floor();
        let want_stalactite = random.next_double() < chance;
        let stalactite_height = match ceiling {
            Some(ceiling) if want_stalactite && blocks.kind(region, at(ceiling)) != blocks.lava => {
                let thickness = self.layer_thickness.sample(random);
                self.replace_with_base(region, at(ceiling), thickness, 1);
                let max = match floor {
                    Some(floor) => cluster_height.min(ceiling - floor),
                    None => cluster_height,
                };
                self.speleothem_height(random, dx, dz, density, max)
            }
            _ => 0,
        };
        let want_stalagmite = random.next_double() < chance;
        let stalagmite_height = match floor {
            Some(floor_y) if want_stalagmite && blocks.kind(region, at(floor_y)) != blocks.lava => {
                let thickness = self.layer_thickness.sample(random);
                self.replace_with_base(region, at(floor_y), thickness, -1);
                if ceiling.is_some() {
                    0.max(stalactite_height + between_inclusive(random, -self.max_height_diff, self.max_height_diff))
                } else {
                    self.speleothem_height(random, dx, dz, density, cluster_height)
                }
            }
            _ => 0,
        };
        let (stalactite, stalagmite) = match (ceiling, floor) {
            (Some(ceiling_y), Some(floor_y)) if ceiling_y - stalactite_height <= floor_y + stalagmite_height => {
                let lowest = (ceiling_y - stalactite_height).max(floor_y + 1);
                let highest = (floor_y + stalagmite_height).min(ceiling_y - 1);
                let bottom = between_inclusive(random, lowest, highest + 1);
                (ceiling_y - bottom, bottom - 1 - floor_y)
            }
            _ => (stalactite_height, stalagmite_height),
        };
        let merge = random.next_boolean() && stalactite > 0 && stalagmite > 0 && column.height().is_some_and(|height| stalactite + stalagmite == height);
        if let Some(ceiling_y) = ceiling {
            blocks.grow(region, at(ceiling_y - 1), Direction::Down, stalactite, merge);
        }
        if let Some(floor_y) = floor {
            blocks.grow(region, at(floor_y + 1), Direction::Up, stalagmite, merge);
        }
    }

    fn speleothem_height(&self, random: &mut dyn RandomSource, dx: i32, dz: i32, density: f32, max: i32) -> i32 {
        if random.next_float() > density {
            return 0;
        }
        let distance = dx.abs() + dz.abs();
        let mean = clamped_map(distance as f64, 0.0, self.max_center_distance as f64, max as f64 / 2.0, 0.0) as f32;
        let value = (mean + random.next_gaussian() as f32 * self.height_deviation as f32).clamp(0.0, max as f32);
        value as i32
    }

    fn can_place_pool(&self, region: &Region, pos: IVec3) -> bool {
        let blocks = &self.blocks;
        let state = region.get(pos);
        let kind = region.blocks.entry(state).kind;
        if kind == blocks.water || region.blocks.same_kind(state, blocks.base) || kind == blocks.pointed_kind {
            return false;
        }
        if fluid_at(region.blocks, region.get(pos + IVec3::Y)) == Some(Fluid::Water) {
            return false;
        }
        let adjacent = |p: IVec3| {
            let state = region.get(p);
            region.blocks.is(state, Tag::BaseStoneOverworld) || fluid_at(region.blocks, state) == Some(Fluid::Water)
        };
        Direction::HORIZONTAL.iter().all(|d| adjacent(pos + d.offset())) && adjacent(pos - IVec3::Y)
    }

    fn replace_with_base(&self, region: &mut Region, first: IVec3, count: i32, step: i32) {
        let mut pos = first;
        for _ in 0..count {
            if !self.blocks.place_base_if_possible(region, pos) {
                return;
            }
            pos.y += step;
        }
    }
}

pub struct LargeDripstone {
    pub blocks: SpeleothemBlocks,
    pub search_range: i32,
    pub column_radius: (i32, i32),
    pub height_scale: FloatProvider,
    pub max_radius_ratio: f32,
    pub stalactite_bluntness: FloatProvider,
    pub stalagmite_bluntness: FloatProvider,
    pub wind_speed: FloatProvider,
    pub min_radius_for_wind: i32,
    pub min_bluntness_for_wind: f32,
}

struct Dripstone {
    root: IVec3,
    up: bool,
    radius: i32,
    bluntness: f64,
    scale: f64,
}

struct Wind {
    origin_y: i32,
    speed: Option<(f64, f64)>,
    max_offset: i32,
}

impl Wind {
    fn offset(&self, pos: IVec3) -> IVec3 {
        let Some((x, z)) = self.speed else { return pos };
        let dy = (self.origin_y - pos.y) as f64;
        let dx = ((x * dy).floor() as i32).clamp(-self.max_offset, self.max_offset);
        let dz = ((z * dy).floor() as i32).clamp(-self.max_offset, self.max_offset);
        pos + IVec3::new(dx, 0, dz)
    }
}

fn speleothem_height(mut distance: f64, radius: f64, scale: f64, bluntness: f64) -> f64 {
    if distance < bluntness {
        distance = bluntness;
    }
    let r = distance / radius * 0.384;
    let part1 = 0.75 * r.powf(1.3333333333333333);
    let part2 = r.powf(0.6666666666666666);
    let part3 = 0.3333333333333333 * r.ln();
    let height = (scale * (part1 - part2 - part3)).max(0.0);
    height / 0.384 * radius
}

impl Dripstone {
    fn height_at(&self, radius: f32) -> i32 {
        speleothem_height(radius as f64, self.radius as f64, self.scale, self.bluntness) as i32
    }

    fn embedded(&self, blocks: &SpeleothemBlocks, region: &Region, center: IVec3) -> bool {
        if blocks.is_empty_or_water_or_lava(region, center) {
            return false;
        }
        let increment = 6.0 / self.radius as f32;
        let mut angle = 0.0f32;
        while angle < std::f32::consts::TAU {
            let dx = (cos(angle as f64) * self.radius as f32) as i32;
            let dz = (sin(angle as f64) * self.radius as f32) as i32;
            if blocks.is_empty_or_water_or_lava(region, center + IVec3::new(dx, 0, dz)) {
                return false;
            }
            angle += increment;
        }
        true
    }

    fn settle(&mut self, blocks: &SpeleothemBlocks, region: &Region, wind: &Wind) -> bool {
        while self.radius > 1 {
            let mut root = self.root;
            let tries = 10.min(self.height_at(0.0));
            for _ in 0..tries {
                if blocks.kind(region, root) == blocks.lava {
                    return false;
                }
                if self.embedded(blocks, region, wind.offset(root)) {
                    self.root = root;
                    return true;
                }
                root.y += if self.up { -1 } else { 1 };
            }
            self.radius /= 2;
        }
        false
    }

    fn place(&self, blocks: &SpeleothemBlocks, region: &mut Region, random: &mut dyn RandomSource, wind: &Wind) {
        for dx in -self.radius..=self.radius {
            for dz in -self.radius..=self.radius {
                let current = ((dx * dx + dz * dz) as f32).sqrt();
                if current > self.radius as f32 {
                    continue;
                }
                let mut height = self.height_at(current);
                if height <= 0 {
                    continue;
                }
                if (random.next_float() as f64) < 0.2 {
                    height = (height as f32 * (random.next_float() * (1.0f32 - 0.8f32) + 0.8)) as i32;
                }
                let mut pos = self.root + IVec3::new(dx, 0, dz);
                let mut out_of_stone = false;
                let max_y = if self.up { region.height(Heightmap::WorldSurfaceWg, pos.x, pos.z) } else { i32::MAX };
                let mut i = 0;
                while i < height && pos.y < max_y {
                    let adjusted = wind.offset(pos);
                    if blocks.is_empty_or_water_or_lava(region, adjusted) {
                        out_of_stone = true;
                        region.set(adjusted, blocks.base);
                    } else if out_of_stone && region.blocks.is(region.get(adjusted), Tag::BaseStoneOverworld) {
                        break;
                    }
                    pos.y += if self.up { 1 } else { -1 };
                    i += 1;
                }
            }
        }
    }

    fn suitable_for_wind(&self, min_radius: i32, min_bluntness: f32) -> bool {
        self.radius >= min_radius && self.bluntness >= min_bluntness as f64
    }
}

impl LargeDripstone {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let region = &mut *ctx.region;
        let blocks = &self.blocks;
        if !blocks.is_empty_or_water(region, origin) {
            return false;
        }
        let base_or_lava = |r: &Region, p: IVec3| {
            let state = r.get(p);
            blocks.is_base(r, state) || r.blocks.entry(state).kind == blocks.lava
        };
        let Some(Column::Range { floor, ceiling }) = Column::scan(region, origin, self.search_range, |r, p| blocks.is_empty_or_water(r, p), base_or_lava) else {
            return false;
        };
        let height = ceiling - floor - 1;
        if height < 4 {
            return false;
        }
        let max_radius = ((height as f32 * self.max_radius_ratio) as i32).clamp(self.column_radius.0, self.column_radius.1);
        let radius = between_inclusive(random, self.column_radius.0, max_radius);
        let mut stalactite = Dripstone {
            root: IVec3::new(origin.x, ceiling - 1, origin.z),
            up: false,
            radius,
            bluntness: self.stalactite_bluntness.sample(random) as f64,
            scale: self.height_scale.sample(random) as f64,
        };
        let mut stalagmite = Dripstone {
            root: IVec3::new(origin.x, floor + 1, origin.z),
            up: true,
            radius,
            bluntness: self.stalagmite_bluntness.sample(random) as f64,
            scale: self.height_scale.sample(random) as f64,
        };
        let wind = if stalactite.suitable_for_wind(self.min_radius_for_wind, self.min_bluntness_for_wind) && stalagmite.suitable_for_wind(self.min_radius_for_wind, self.min_bluntness_for_wind) {
            let speed = self.wind_speed.sample(random);
            let direction = random.next_float() * std::f32::consts::PI;
            Wind {
                origin_y: origin.y,
                speed: Some(((cos(direction as f64) * speed) as f64, (sin(direction as f64) * speed) as f64)),
                max_offset: 16 - radius,
            }
        } else {
            Wind {
                origin_y: 0,
                speed: None,
                max_offset: 0,
            }
        };
        let stalactite_embedded = stalactite.settle(blocks, region, &wind);
        let stalagmite_embedded = stalagmite.settle(blocks, region, &wind);
        if stalactite_embedded {
            stalactite.place(blocks, region, random, &wind);
        }
        if stalagmite_embedded {
            stalagmite.place(blocks, region, random, &wind);
        }
        true
    }
}
