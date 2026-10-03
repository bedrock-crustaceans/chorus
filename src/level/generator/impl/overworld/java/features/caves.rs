use glam::IVec3;

use super::super::blocks::{BlockId, BlockTable, flag, int, key, state, text};
use super::super::noise::{NoiseStack, NormalNoise};
use super::super::proto::BlockEntity;
use super::super::random::RandomSource;
use super::super::tags::Tag;
use super::basic::simple;
use super::key::Key;
use super::multiface::{MultifaceGrowth, MultifaceStates, SculkBlocks};
use super::placement::{Modifier, Placed, count, filter, vertical_offset};
use super::predicate::{BlockPredicate, Direction, Fluid, can_survive, empty, fluid_at};
use super::provider::{Anchor, BOTTOM, FloatProvider, HeightProvider, IntProvider, StateProvider, TOP};
use super::region::{Heightmap, Region};
use super::sculk::SculkPatch;
use super::speleothem::{LargeDripstone, Speleothem, SpeleothemBlocks, SpeleothemCluster};
use super::vegetation::{self, BlockColumn, VegetationPatch};
use super::{Context, Feature, trees};
use crate::block::block_id::*;
use crate::block::state::common::{BIG_DRIPLEAF_HEAD, CAN_SUMMON, GROWING_PLANT_AGE, MINECRAFT_BLOCK_FACE, MINECRAFT_CARDINAL_DIRECTION, UPPER_BLOCK_BIT};

const DUNGEON_MOBS: [(&str, f32, f32); 4] = [
    ("minecraft:skeleton", 0.6, 1.99),
    ("minecraft:zombie", 0.6, 1.95),
    ("minecraft:zombie", 0.6, 1.95),
    ("minecraft:spider", 1.4, 0.9),
];

pub struct MonsterRoom {
    pub air: BlockId,
    pub cobblestone: BlockId,
    pub mossy: BlockId,
    pub chests: [BlockId; 4],
    pub spawner: BlockId,
    pub chest_kind: u16,
    pub spawner_kind: u16,
}

fn solid(region: &Region, pos: IVec3) -> bool {
    region.blocks.entry(region.get(pos)).solid
}

fn full(region: &Region, pos: IVec3) -> bool {
    region.blocks.entry(region.get(pos)).full
}

fn is_air(region: &Region, pos: IVec3) -> bool {
    region.blocks.is_air(region.get(pos))
}

impl MonsterRoom {
    fn safe_set(region: &mut Region, pos: IVec3, state: BlockId) {
        if !region.blocks.is(region.get(pos), Tag::FeaturesCannotReplace) {
            region.set(pos, state);
        }
    }

    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let region = &mut *ctx.region;
        let xr = random.next_int_bounded(2) + 2;
        let (min_x, max_x) = (-xr - 1, xr + 1);
        let zr = random.next_int_bounded(2) + 2;
        let (min_z, max_z) = (-zr - 1, zr + 1);
        let mut holes = 0;
        for dx in min_x..=max_x {
            for dy in -1..=4 {
                for dz in min_z..=max_z {
                    let pos = origin + IVec3::new(dx, dy, dz);
                    let is_solid = solid(region, pos);
                    if (dy == -1 || dy == 4) && !is_solid {
                        return false;
                    }
                    if (dx == min_x || dx == max_x || dz == min_z || dz == max_z) && dy == 0 && is_air(region, pos) && is_air(region, pos + IVec3::Y) {
                        holes += 1;
                    }
                }
            }
        }
        if !(1..=5).contains(&holes) {
            return false;
        }
        for dx in min_x..=max_x {
            for dy in (-1..=3).rev() {
                for dz in min_z..=max_z {
                    let pos = origin + IVec3::new(dx, dy, dz);
                    let kind = region.blocks.entry(region.get(pos)).kind;
                    if dx == min_x || dy == -1 || dz == min_z || dx == max_x || dz == max_z {
                        if pos.y >= region.min_y() && !solid(region, pos - IVec3::Y) {
                            region.set(pos, self.air);
                        } else if solid(region, pos) && kind != self.chest_kind {
                            let state = if dy == -1 && random.next_int_bounded(4) != 0 { self.mossy } else { self.cobblestone };
                            Self::safe_set(region, pos, state);
                        }
                    } else if kind != self.chest_kind && kind != self.spawner_kind {
                        Self::safe_set(region, pos, self.air);
                    }
                }
            }
        }
        for _ in 0..2 {
            for _ in 0..3 {
                let x = origin.x + random.next_int_bounded(xr * 2 + 1) - xr;
                let z = origin.z + random.next_int_bounded(zr * 2 + 1) - zr;
                let pos = IVec3::new(x, origin.y, z);
                if !is_air(region, pos) {
                    continue;
                }
                let walls: Vec<usize> = Direction::HORIZONTAL.iter().enumerate().filter(|(_, d)| solid(region, pos + d.offset())).map(|(i, _)| i).collect();
                if walls.len() == 1 {
                    let facing = Direction::HORIZONTAL[walls[0]].opposite();
                    let index = Direction::HORIZONTAL.iter().position(|&d| d == facing).unwrap_or(0);
                    Self::safe_set(region, pos, self.chests[index]);
                    let seed = random.next_long();
                    region.set_block_entity(pos, BlockEntity::Chest { loot_table: "simple_dungeon", seed });
                    break;
                }
            }
        }
        Self::safe_set(region, origin, self.spawner);
        let (entity, width, height) = DUNGEON_MOBS[random.next_int_bounded(DUNGEON_MOBS.len() as i32) as usize];
        region.set_block_entity(origin, BlockEntity::Spawner { entity, width, height });
        true
    }
}

pub struct UnderwaterMagma {
    pub water: u16,
    pub magma: BlockId,
    pub floor_search_range: i32,
    pub radius: i32,
    pub probability: f32,
}

impl UnderwaterMagma {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let region = &mut *ctx.region;
        let water = |r: &Region, p: IVec3| r.blocks.entry(r.get(p)).kind == self.water;
        if !water(region, origin) {
            return false;
        }
        let mut cursor = origin;
        let mut i = 1;
        while i < self.floor_search_range && water(region, cursor) {
            cursor.y -= 1;
            i += 1;
        }
        if water(region, cursor) {
            return false;
        }
        let floor = IVec3::new(origin.x, cursor.y, origin.z);
        let mut placed = 0;
        for z in -self.radius..=self.radius {
            for y in -self.radius..=self.radius {
                for x in -self.radius..=self.radius {
                    let pos = floor + IVec3::new(x, y, z);
                    if random.next_float() < self.probability && self.valid(region, pos) {
                        region.set(pos, self.magma);
                        placed += 1;
                    }
                }
            }
        }
        placed > 0
    }

    fn valid(&self, region: &Region, pos: IVec3) -> bool {
        let state = region.get(pos);
        if region.blocks.is_air(state) || region.blocks.entry(state).kind == self.water || !full(region, pos - IVec3::Y) {
            return false;
        }
        Direction::HORIZONTAL.iter().all(|d| full(region, pos + d.offset()))
    }
}

pub struct RootSystem {
    pub tree: Box<Placed>,
    pub required_vertical_space: i32,
    pub root_radius: i32,
    pub root_replaceable: Tag,
    pub root: BlockId,
    pub root_attempts: i32,
    pub column_max_height: i32,
    pub hanging_radius: i32,
    pub hanging_span: i32,
    pub hanging: BlockId,
    pub hanging_attempts: i32,
    pub allowed_vertical_water: i32,
    pub allowed_tree_position: BlockPredicate,
}

impl RootSystem {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        if !is_air(ctx.region, origin) {
            return false;
        }
        let mut working = origin;
        if self.place_dirt_and_tree(ctx, random, &mut working, origin) {
            self.place_roots(ctx.region, random, origin);
        }
        true
    }

    fn space_for_tree(&self, region: &Region, pos: IVec3) -> bool {
        for i in 1..=self.required_vertical_space {
            let state = region.get(pos + IVec3::Y * i);
            if region.blocks.is_air(state) {
                continue;
            }
            if i + 1 > self.allowed_vertical_water || fluid_at(region.blocks, state) != Some(Fluid::Water) {
                return false;
            }
        }
        true
    }

    fn place_dirt_and_tree(&self, ctx: &mut Context, random: &mut dyn RandomSource, working: &mut IVec3, origin: IVec3) -> bool {
        for y in 0..self.column_max_height {
            *working += IVec3::Y;
            if ctx.region.height(Heightmap::WorldSurface, working.x, working.z) < working.y {
                return false;
            }
            if self.allowed_tree_position.test(ctx.region, *working) && self.space_for_tree(ctx.region, *working) {
                let below = *working - IVec3::Y;
                if fluid_at(ctx.region.blocks, ctx.region.get(below)) == Some(Fluid::Lava) || !solid(ctx.region, below) {
                    return false;
                }
                if self.tree.place(ctx, random, *working, None) {
                    for dirt_y in origin.y..origin.y + y {
                        self.place_rooted_dirt(ctx.region, random, IVec3::new(origin.x, dirt_y, origin.z));
                    }
                    return true;
                }
            }
        }
        false
    }

    fn place_rooted_dirt(&self, region: &mut Region, random: &mut dyn RandomSource, center: IVec3) {
        for _ in 0..self.root_attempts {
            let dx = random.next_int_bounded(self.root_radius) - random.next_int_bounded(self.root_radius);
            let dz = random.next_int_bounded(self.root_radius) - random.next_int_bounded(self.root_radius);
            let pos = center + IVec3::new(dx, 0, dz);
            if region.blocks.is(region.get(pos), self.root_replaceable) {
                region.set(pos, self.root);
            }
        }
    }

    fn place_roots(&self, region: &mut Region, random: &mut dyn RandomSource, origin: IVec3) {
        for _ in 0..self.hanging_attempts {
            let dx = random.next_int_bounded(self.hanging_radius) - random.next_int_bounded(self.hanging_radius);
            let dy = random.next_int_bounded(self.hanging_span) - random.next_int_bounded(self.hanging_span);
            let dz = random.next_int_bounded(self.hanging_radius) - random.next_int_bounded(self.hanging_radius);
            let pos = origin + IVec3::new(dx, dy, dz);
            if is_air(region, pos) && can_survive(region, self.hanging, pos) && full(region, pos + IVec3::Y) {
                region.set(pos, self.hanging);
            }
        }
    }
}

pub struct Geode {
    pub noise: NoiseStack,
    pub inner: BlockId,
    pub alternate_inner: BlockId,
    pub middle: BlockId,
    pub outer: BlockId,
    pub buds: Vec<[[BlockId; 2]; 6]>,
    pub water: u16,
}

const GEODE_FILLING: f64 = 1.7;
const GEODE_INNER: f64 = 2.2;
const GEODE_MIDDLE: f64 = 3.2;
const GEODE_OUTER: f64 = 4.2;
const GEODE_CRACK_CHANCE: f64 = 0.95;
const GEODE_CRACK_SIZE: f64 = 2.0;
const GEODE_CRACK_POINT_OFFSET: i32 = 2;
const GEODE_POTENTIAL_PLACEMENTS: f64 = 0.35;
const GEODE_ALTERNATE_CHANCE: f64 = 0.083;
const GEODE_GEN_OFFSET: i32 = 16;
const GEODE_NOISE_MULTIPLIER: f64 = 0.05;
const GEODE_INVALID_THRESHOLD: i32 = 1;

impl Geode {
    fn safe_set(region: &mut Region, pos: IVec3, state: BlockId) {
        if !region.blocks.is(region.get(pos), Tag::FeaturesCannotReplace) {
            region.set(pos, state);
        }
    }

    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let region = &mut *ctx.region;
        let outer_wall = IntProvider::Uniform(4, 6);
        let point_offset = IntProvider::Uniform(1, 2);
        let points_count = IntProvider::Uniform(3, 4).sample(random);
        let crack_adjustment = points_count as f64 / 6.0;
        let inner_air = 1.0 / GEODE_FILLING.sqrt();
        let innermost = 1.0 / (GEODE_INNER + crack_adjustment).sqrt();
        let inner_crust = 1.0 / (GEODE_MIDDLE + crack_adjustment).sqrt();
        let outer_crust = 1.0 / (GEODE_OUTER + crack_adjustment).sqrt();
        let crack_size = 1.0 / (GEODE_CRACK_SIZE + random.next_double() / 2.0 + if points_count > 3 { crack_adjustment } else { 0.0 }).sqrt();
        let crack = (random.next_float() as f64) < GEODE_CRACK_CHANCE;
        let mut invalid = 0;
        let mut points = Vec::new();
        for _ in 0..points_count {
            let x = outer_wall.sample(random);
            let y = outer_wall.sample(random);
            let z = outer_wall.sample(random);
            let pos = origin + IVec3::new(x, y, z);
            let state = region.get(pos);
            if region.blocks.is_air(state) || region.blocks.is(state, Tag::GeodeInvalidBlocks) {
                invalid += 1;
                if invalid > GEODE_INVALID_THRESHOLD {
                    return false;
                }
            }
            points.push((pos, point_offset.sample(random)));
        }
        let mut crack_points = Vec::new();
        if crack {
            let index = random.next_int_bounded(4);
            let offset = points_count * 2 + 1;
            let (cx, cz) = match index {
                0 => (offset, 0),
                1 => (0, offset),
                2 => (offset, offset),
                _ => (0, 0),
            };
            for y in [7, 5, 1] {
                crack_points.push(origin + IVec3::new(cx, y, cz));
            }
        }
        let mut potential = Vec::new();
        for z in -GEODE_GEN_OFFSET..=GEODE_GEN_OFFSET {
            for y in -GEODE_GEN_OFFSET..=GEODE_GEN_OFFSET {
                for x in -GEODE_GEN_OFFSET..=GEODE_GEN_OFFSET {
                    let pos = origin + IVec3::new(x, y, z);
                    let noise = self.noise.get(pos.x as f64, pos.y as f64, pos.z as f64) as f64 * GEODE_NOISE_MULTIPLIER;
                    let shell: f64 = points.iter().map(|&(point, offset)| 1.0 / (((pos - point).length_squared() + offset) as f64).sqrt() + noise).sum();
                    if shell < outer_crust {
                        continue;
                    }
                    if shell >= inner_air {
                        Self::safe_set(region, pos, super::super::blocks::AIR);
                        continue;
                    }
                    let crack_sum: f64 = crack_points
                        .iter()
                        .map(|&point| 1.0 / (((pos - point).length_squared() + GEODE_CRACK_POINT_OFFSET) as f64).sqrt() + noise)
                        .sum();
                    if crack && crack_sum >= crack_size {
                        Self::safe_set(region, pos, super::super::blocks::AIR);
                    } else if shell >= innermost {
                        let alternate = (random.next_float() as f64) < GEODE_ALTERNATE_CHANCE;
                        Self::safe_set(region, pos, if alternate { self.alternate_inner } else { self.inner });
                        if alternate && (random.next_float() as f64) < GEODE_POTENTIAL_PLACEMENTS {
                            potential.push(pos);
                        }
                    } else if shell >= inner_crust {
                        Self::safe_set(region, pos, self.middle);
                    } else {
                        Self::safe_set(region, pos, self.outer);
                    }
                }
            }
        }
        for crystal in potential {
            let bud = &self.buds[random.next_int_bounded(self.buds.len() as i32) as usize];
            for (index, direction) in Direction::ALL.into_iter().enumerate() {
                let pos = crystal + direction.offset();
                let state = region.get(pos);
                let wet = region.blocks.entry(state).kind == self.water;
                if region.blocks.is_air(state) || wet {
                    Self::safe_set(region, pos, bud[index][wet as usize]);
                    break;
                }
            }
        }
        true
    }
}

fn kinds(table: &mut BlockTable, names: &[&'static str]) -> Vec<u16> {
    names.iter().map(|name| table.kind(name)).collect()
}

fn air_or_water(table: &mut BlockTable) -> BlockPredicate {
    BlockPredicate::AnyOf(vec![empty(), BlockPredicate::MatchingBlocks(IVec3::ZERO, vec![table.kind(WATER)])])
}

fn scan(direction: Direction, target: BlockPredicate, allowed: BlockPredicate) -> Modifier {
    Modifier::EnvironmentScan {
        direction,
        target,
        allowed,
        max_steps: 12,
    }
}

fn range_to_max_terrain() -> Modifier {
    Modifier::HeightRange(HeightProvider::Uniform(BOTTOM, Anchor::Absolute(256)))
}

fn speleothem(table: &mut BlockTable, base: &'static str, pointed: &'static str, replaceable: Tag) -> Feature {
    let mut placed = Vec::new();
    for (direction, offset) in [(Direction::Down, 1), (Direction::Up, -1)] {
        let feature = Feature::Speleothem(Speleothem {
            blocks: SpeleothemBlocks::new(table, base, pointed, replaceable),
            taller_chance: 0.2,
            directional_spread: 0.7,
            spread_radius2: 0.5,
            spread_radius3: 0.5,
        });
        let allowed = air_or_water(table);
        placed.push(Placed::new(feature, vec![scan(direction, BlockPredicate::Solid(IVec3::ZERO), allowed), vertical_offset(offset.into())]));
    }
    Feature::SimpleRandomSelector(placed)
}

fn speleothem_placement() -> Vec<Modifier> {
    let normal = |deviation: f32, bound: i32| IntProvider::ClampedNormal {
        mean: 0.0,
        deviation,
        min: -bound,
        max: bound,
    };
    vec![
        Modifier::Count(IntProvider::Uniform(192, 256)),
        Modifier::InSquare,
        range_to_max_terrain(),
        Modifier::Count(IntProvider::Uniform(1, 5)),
        Modifier::Offset(normal(3.0, 10), normal(0.6, 2), normal(3.0, 10)),
        Modifier::Biome,
    ]
}

fn cluster(table: &mut BlockTable, base: &'static str, pointed: &'static str, replaceable: Tag, height: IntProvider, wetness: FloatProvider) -> Feature {
    Feature::SpeleothemCluster(SpeleothemCluster {
        blocks: SpeleothemBlocks::new(table, base, pointed, replaceable),
        search_range: 12,
        height,
        radius: IntProvider::Uniform(2, 8),
        max_height_diff: 1,
        height_deviation: 3,
        layer_thickness: IntProvider::Uniform(2, 4),
        density: FloatProvider::Uniform(0.3, 0.7),
        wetness,
        chance_at_max_distance: 0.1,
        max_edge_distance: 3,
        max_center_distance: 8,
    })
}

fn cave_vines(table: &mut BlockTable, body_height: IntProvider) -> Feature {
    let body = StateProvider::Weighted(vec![(table.name(CAVE_VINES), 4), (table.name(CAVE_VINES_BODY_WITH_BERRIES), 1)]);
    for age in 23..=25 {
        table.get(CAVE_VINES, &[state(GROWING_PLANT_AGE, int(age))]);
        table.get(CAVE_VINES_HEAD_WITH_BERRIES, &[state(GROWING_PLANT_AGE, int(age))]);
    }
    let head_source = StateProvider::Weighted(vec![(table.name(CAVE_VINES), 4), (table.name(CAVE_VINES_HEAD_WITH_BERRIES), 1)]);
    let head = StateProvider::RandomizedInt {
        source: Box::new(head_source),
        property: key(GROWING_PLANT_AGE),
        values: IntProvider::Uniform(23, 25),
    };
    Feature::BlockColumn(BlockColumn {
        layers: vec![(body_height, body), (1.into(), head)],
        direction: Direction::Down,
        allowed: empty(),
        prioritize_tip: true,
    })
}

fn moss_patch(table: &mut BlockTable, ceiling: bool) -> Feature {
    let vegetation = if ceiling {
        cave_vines(table, IntProvider::Weighted(vec![(IntProvider::Uniform(0, 3), 5), (IntProvider::Uniform(1, 7), 1)]))
    } else {
        simple(vegetation::weighted(
            table,
            &[(FLOWERING_AZALEA, 4), (AZALEA, 7), (MOSS_CARPET, 25), (SHORT_GRASS, 50), (TALL_GRASS, 10)],
        ))
    };
    Feature::VegetationPatch(VegetationPatch {
        replaceable: Tag::MossReplaceable,
        ground: table.name(MOSS_BLOCK).into(),
        vegetation: Box::new(Placed::new(vegetation, Vec::new())),
        ceiling,
        depth: if ceiling { IntProvider::Uniform(1, 2) } else { 1.into() },
        extra_bottom_chance: 0.0,
        vertical_range: 5,
        vegetation_chance: if ceiling { 0.08 } else { 0.8 },
        xz_radius: IntProvider::Uniform(4, 7),
        extra_edge_chance: 0.3,
        water: None,
    })
}

fn dripleaf(table: &mut BlockTable) -> Feature {
    let mut small = Vec::new();
    for direction in [Direction::East, Direction::West, Direction::North, Direction::South] {
        let facing = state(MINECRAFT_CARDINAL_DIRECTION, text(direction.name()));
        let upper = table.get(SMALL_DRIPLEAF_BLOCK, &[facing.clone(), state(UPPER_BLOCK_BIT, flag(true))]);
        let lower = table.get(SMALL_DRIPLEAF_BLOCK, &[facing, state(UPPER_BLOCK_BIT, flag(false))]);
        table.flooded(upper);
        table.flooded(lower);
        small.push((lower, 1));
    }
    let mut features = vec![Placed::new(simple(StateProvider::Weighted(small)), Vec::new())];
    for direction in [Direction::East, Direction::West, Direction::South, Direction::North] {
        let state = |table: &mut BlockTable, head: bool| {
            let dry = table.get(BIG_DRIPLEAF, &[state(BIG_DRIPLEAF_HEAD, flag(head)), state(MINECRAFT_CARDINAL_DIRECTION, text(direction.name()))]);
            table.flooded(dry);
            dry
        };
        let stem = state(table, false);
        let head = state(table, true);
        let column = Feature::BlockColumn(BlockColumn {
            layers: vec![(IntProvider::Weighted(vec![(IntProvider::Uniform(0, 4), 2), (0.into(), 1)]), stem.into()), (1.into(), head.into())],
            direction: Direction::Up,
            allowed: air_or_water(table),
            prioritize_tip: true,
        });
        features.push(Placed::new(column, Vec::new()));
    }
    Feature::SimpleRandomSelector(features)
}

fn clay_patch(table: &mut BlockTable, pool: bool) -> Feature {
    Feature::VegetationPatch(VegetationPatch {
        replaceable: Tag::LushGroundReplaceable,
        ground: table.name(CLAY).into(),
        vegetation: Box::new(Placed::new(dripleaf(table), Vec::new())),
        ceiling: false,
        depth: 3.into(),
        extra_bottom_chance: 0.8,
        vertical_range: if pool { 5 } else { 2 },
        vegetation_chance: if pool { 0.1 } else { 0.05 },
        xz_radius: IntProvider::Uniform(4, 7),
        extra_edge_chance: 0.7,
        water: pool.then(|| table.name(WATER)),
    })
}

fn multiface(table: &mut BlockTable, name: &'static str, sculk: bool, floor: bool, chance: f32, on: &[&'static str]) -> Feature {
    let sculk = sculk.then(|| SculkBlocks {
        sculk: table.kind(SCULK),
        catalyst: table.kind(SCULK_CATALYST),
        moving_block: table.kind(MOVING_BLOCK),
    });
    Feature::MultifaceGrowth(MultifaceGrowth {
        states: MultifaceStates::register(table, name),
        sculk,
        search_range: 20,
        floor,
        ceiling: true,
        wall: true,
        chance_of_spreading: chance,
        can_be_placed_on: kinds(table, on),
    })
}

const LICHEN_SURFACES: &[&str] = &[STONE, ANDESITE, DIORITE, GRANITE, DRIPSTONE_BLOCK, CALCITE, TUFF, DEEPSLATE, SULFUR, CINNABAR];

fn geode(table: &mut BlockTable, seed: i64) -> Feature {
    let buds = [SMALL_AMETHYST_BUD, MEDIUM_AMETHYST_BUD, LARGE_AMETHYST_BUD, AMETHYST_CLUSTER]
        .iter()
        .map(|name| {
            Direction::ALL.map(|direction| {
                let dry = table.get(name, &[state(MINECRAFT_BLOCK_FACE, text(direction.name()))]);
                [dry, table.flooded(dry)]
            })
        })
        .collect();
    Feature::Geode(Geode {
        noise: NormalNoise::parity(-4, &[1.0]).create_legacy(seed),
        inner: table.name(AMETHYST_BLOCK),
        alternate_inner: table.name(BUDDING_AMETHYST),
        middle: table.name(CALCITE),
        outer: table.name(SMOOTH_BASALT),
        buds,
        water: table.kind(WATER),
    })
}

fn sculk_patch(table: &mut BlockTable) -> Feature {
    let patch = Feature::SculkPatch(Box::new(SculkPatch {
        vein: MultifaceStates::register(table, SCULK_VEIN),
        blocks: SculkBlocks {
            sculk: table.kind(SCULK),
            catalyst: table.kind(SCULK_CATALYST),
            moving_block: table.kind(MOVING_BLOCK),
        },
        sculk: table.name(SCULK),
        water: table.name(WATER),
        sensor: {
            let sensor = table.name(SCULK_SENSOR);
            table.flooded(sensor);
            sensor
        },
        shrieker: {
            let shrieker = table.get(SCULK_SHRIEKER, &[state(CAN_SUMMON, flag(true))]);
            table.flooded(shrieker);
            shrieker
        },
        inhibitors: kinds(table, &[SCULK_SENSOR, SCULK_SHRIEKER]),
        charge_count: 10,
        amount_per_charge: 32,
        spread_attempts: 64,
        growth_rounds: 0,
        spread_rounds: 1,
    }));
    let catalyst = Placed::new(simple(table.name(SCULK_CATALYST)), vec![Modifier::RandomChance(0.5), filter(BlockPredicate::SturdyFace(IVec3::NEG_Y))]);
    Feature::Sequence(vec![Placed::new(patch, Vec::new()), catalyst])
}

fn rooted_azalea(table: &mut BlockTable) -> Feature {
    let allowed = BlockPredicate::AllOf(vec![
        BlockPredicate::AnyOf(vec![empty(), BlockPredicate::MatchingTag(IVec3::ZERO, Tag::ReplaceableByTrees)]),
        BlockPredicate::MatchingTag(IVec3::NEG_Y, Tag::AzaleaGrowsOn),
    ]);
    Feature::RootSystem(RootSystem {
        tree: Box::new(trees::inline(table, trees::Species::Azalea)),
        required_vertical_space: 3,
        root_radius: 3,
        root_replaceable: Tag::AzaleaRootReplaceable,
        root: table.name(DIRT_WITH_ROOTS),
        root_attempts: 20,
        column_max_height: 100,
        hanging_radius: 3,
        hanging_span: 2,
        hanging: table.name(HANGING_ROOTS),
        hanging_attempts: 20,
        allowed_vertical_water: 2,
        allowed_tree_position: allowed,
    })
}

pub fn define(key: Key, table: &mut BlockTable, seed: i64) -> Option<Placed> {
    let below_ceiling = |target: BlockPredicate| vec![scan(Direction::Up, target, empty()), vertical_offset((-1).into())];
    let above_floor = || vec![scan(Direction::Down, BlockPredicate::Solid(IVec3::ZERO), empty()), vertical_offset(1.into())];
    let in_caves = |prefix: Vec<Modifier>, tail: Vec<Modifier>| {
        let mut modifiers = prefix;
        modifiers.extend([Modifier::InSquare, range_to_max_terrain()]);
        modifiers.extend(tail);
        modifiers.push(Modifier::Biome);
        modifiers
    };
    let (feature, placement) = match key {
        Key::MonsterRoom | Key::MonsterRoomDeep => {
            let feature = Feature::MonsterRoom(MonsterRoom {
                air: table.name(AIR),
                cobblestone: table.name(COBBLESTONE),
                mossy: table.name(MOSSY_COBBLESTONE),
                chests: Direction::HORIZONTAL.map(|d| table.get(CHEST, &[state(MINECRAFT_CARDINAL_DIRECTION, text(d.name()))])),
                spawner: table.name(MOB_SPAWNER),
                chest_kind: table.kind(CHEST),
                spawner_kind: table.kind(MOB_SPAWNER),
            });
            let (tries, range) = if key == Key::MonsterRoom {
                (10, HeightProvider::Uniform(Anchor::Absolute(0), TOP))
            } else {
                (4, HeightProvider::Uniform(Anchor::AboveBottom(6), Anchor::Absolute(-1)))
            };
            (feature, vec![count(tries), Modifier::InSquare, Modifier::HeightRange(range), Modifier::Biome])
        }
        Key::DripstoneCluster => (
            cluster(
                table,
                DRIPSTONE_BLOCK,
                POINTED_DRIPSTONE,
                Tag::DripstoneReplaceableBlocks,
                IntProvider::Uniform(3, 6),
                FloatProvider::ClampedNormal {
                    mean: 0.1,
                    deviation: 0.3,
                    min: 0.1,
                    max: 0.9,
                },
            ),
            in_caves(vec![Modifier::Count(IntProvider::Uniform(48, 96))], Vec::new()),
        ),
        Key::SulfurSpikeCluster => (
            cluster(table, SULFUR, SULFUR_SPIKE, Tag::SulfurSpikeReplaceableBlocks, IntProvider::Uniform(1, 4), FloatProvider::Constant(0.0)),
            in_caves(vec![Modifier::Count(IntProvider::Uniform(48, 96))], Vec::new()),
        ),
        Key::LargeDripstone => {
            let feature = Feature::LargeDripstone(LargeDripstone {
                blocks: SpeleothemBlocks::new(table, DRIPSTONE_BLOCK, POINTED_DRIPSTONE, Tag::DripstoneReplaceableBlocks),
                search_range: 30,
                column_radius: (3, 16),
                height_scale: FloatProvider::Uniform(0.4, 2.0),
                max_radius_ratio: 0.33,
                stalactite_bluntness: FloatProvider::Uniform(0.3, 0.9),
                stalagmite_bluntness: FloatProvider::Uniform(0.4, 1.0),
                wind_speed: FloatProvider::Uniform(0.0, 0.3),
                min_radius_for_wind: 4,
                min_bluntness_for_wind: 0.6,
            });
            (feature, in_caves(vec![Modifier::Count(IntProvider::Uniform(10, 48))], Vec::new()))
        }
        Key::PointedDripstone => (speleothem(table, DRIPSTONE_BLOCK, POINTED_DRIPSTONE, Tag::DripstoneReplaceableBlocks), speleothem_placement()),
        Key::SulfurSpike => (speleothem(table, SULFUR, SULFUR_SPIKE, Tag::SulfurSpikeReplaceableBlocks), speleothem_placement()),
        Key::UnderwaterMagma => {
            let feature = Feature::UnderwaterMagma(UnderwaterMagma {
                water: table.kind(WATER),
                magma: table.name(MAGMA),
                floor_search_range: 5,
                radius: 1,
                probability: 0.5,
            });
            let threshold = Modifier::SurfaceRelativeThreshold {
                heightmap: Heightmap::OceanFloorWg,
                min: i32::MIN,
                max: -2,
            };
            (feature, in_caves(vec![Modifier::Count(IntProvider::Uniform(44, 52))], vec![threshold]))
        }
        Key::GlowLichen => {
            let feature = multiface(table, GLOW_LICHEN, false, false, 0.5, LICHEN_SURFACES);
            let threshold = Modifier::SurfaceRelativeThreshold {
                heightmap: Heightmap::OceanFloorWg,
                min: i32::MIN,
                max: -13,
            };
            (
                feature,
                vec![Modifier::Count(IntProvider::Uniform(104, 157)), range_to_max_terrain(), Modifier::InSquare, threshold, Modifier::Biome],
            )
        }
        Key::SculkVein => (
            multiface(table, SCULK_VEIN, true, true, 1.0, &LICHEN_SURFACES[..8]),
            in_caves(vec![Modifier::Count(IntProvider::Uniform(204, 250))], Vec::new()),
        ),
        Key::RootedAzaleaTree => (
            rooted_azalea(table),
            in_caves(vec![Modifier::Count(IntProvider::Uniform(1, 2))], below_ceiling(BlockPredicate::Solid(IVec3::ZERO))),
        ),
        Key::CaveVines => {
            let body = IntProvider::Weighted(vec![(IntProvider::Uniform(0, 19), 2), (IntProvider::Uniform(0, 2), 3), (IntProvider::Uniform(0, 6), 10)]);
            (cave_vines(table, body), in_caves(vec![count(188)], below_ceiling(BlockPredicate::SturdyFace(IVec3::ZERO))))
        }
        Key::LushCavesVegetation => (moss_patch(table, false), in_caves(vec![count(125)], above_floor())),
        Key::LushCavesClay => {
            let feature = Feature::RandomBoolean(Box::new(Placed::new(clay_patch(table, false), Vec::new())), Box::new(Placed::new(clay_patch(table, true), Vec::new())));
            (feature, in_caves(vec![count(62)], above_floor()))
        }
        Key::LushCavesCeilingVegetation => (moss_patch(table, true), in_caves(vec![count(125)], below_ceiling(BlockPredicate::Solid(IVec3::ZERO)))),
        Key::SporeBlossom => (simple(table.name(SPORE_BLOSSOM)), in_caves(vec![count(25)], below_ceiling(BlockPredicate::Solid(IVec3::ZERO)))),
        Key::ClassicVines => (vegetation::vines(table), in_caves(vec![count(256)], Vec::new())),
        Key::AmethystGeode => (
            geode(table, seed),
            vec![
                Modifier::Rarity(24),
                Modifier::InSquare,
                Modifier::HeightRange(HeightProvider::Uniform(Anchor::AboveBottom(6), Anchor::Absolute(30))),
                Modifier::Biome,
            ],
        ),
        Key::SculkPatchDeepDark => (sculk_patch(table), in_caves(vec![count(256)], Vec::new())),
        _ => return None,
    };
    Some(Placed::new(feature, placement))
}
