use glam::IVec3;

use super::super::blocks::{BlockId, BlockTable, flag, int, state, text};
use super::super::noise::NormalNoise;
use super::super::random::RandomSource;
use super::super::tags::Tag;
use super::basic::simple;
use super::key::Key;
use super::placement::{Modifier, Placed, count, count_extra, filter};
use super::predicate::{BlockPredicate, Direction, Fluid, can_survive, empty, vine_bit};
use super::provider::{IntProvider, StateProvider};
use super::region::Heightmap;
use super::tree::PositionSet;
use super::trees::{self, Fallen, Species, TreeFeature};
use super::{Context, Feature};
use chorus_block::block_id::*;
use chorus_block::state::common::{AGE_BIT, BAMBOO_LEAF_SIZE, BAMBOO_STALK_THICKNESS, GROWTH, MINECRAFT_CARDINAL_DIRECTION, UPPER_BLOCK_BIT, VINE_DIRECTION_BITS};

pub struct BlockColumn {
    pub layers: Vec<(IntProvider, StateProvider)>,
    pub direction: Direction,
    pub allowed: BlockPredicate,
    pub prioritize_tip: bool,
}

impl BlockColumn {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let mut heights: Vec<i32> = self.layers.iter().map(|(height, _)| height.sample(random)).collect();
        let total: i32 = heights.iter().sum();
        if total == 0 {
            return false;
        }
        let step = self.direction.offset();
        let mut next = origin + step;
        for y in 0..total {
            if !self.allowed.test(ctx.region, next) {
                truncate(&mut heights, total, y, self.prioritize_tip);
                break;
            }
            next += step;
        }
        let mut pos = origin;
        for ((_, state), &count) in self.layers.iter().zip(&heights) {
            for _ in 0..count {
                let block = state.state(ctx.region, random, pos);
                ctx.region.set(pos, block);
                pos += step;
            }
        }
        true
    }
}

fn truncate(heights: &mut [i32], total: i32, new_height: i32, prioritize_tip: bool) {
    let mut remove = total - new_height;
    let indices: Vec<usize> = if prioritize_tip { (0..heights.len()).collect() } else { (0..heights.len()).rev().collect() };
    for i in indices {
        if remove <= 0 {
            break;
        }
        let taken = heights[i].min(remove);
        remove -= taken;
        heights[i] -= taken;
    }
}

pub struct Bamboo {
    pub probability: f32,
    pub trunk: BlockId,
    pub final_large: BlockId,
    pub top_large: BlockId,
    pub top_small: BlockId,
    pub podzol: BlockId,
}

impl Bamboo {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let blocks = ctx.region.blocks;
        if !blocks.is_air(ctx.region.get(origin)) {
            return false;
        }
        if can_survive(ctx.region, self.trunk, origin) {
            let height = random.next_int_bounded(12) + 5;
            if random.next_float() < self.probability {
                let r = random.next_int_bounded(4) + 1;
                for xx in origin.x - r..=origin.x + r {
                    for zz in origin.z - r..=origin.z + r {
                        let (xd, zd) = (xx - origin.x, zz - origin.z);
                        if xd * xd + zd * zd <= r * r {
                            let pos = IVec3::new(xx, ctx.region.height(Heightmap::WorldSurface, xx, zz) - 1, zz);
                            if blocks.is(ctx.region.get(pos), Tag::BeneathBambooPodzolReplaceable) {
                                ctx.region.set(pos, self.podzol);
                            }
                        }
                    }
                }
            }
            let mut pos = origin;
            let mut i = 0;
            while i < height && blocks.is_air(ctx.region.get(pos)) {
                ctx.region.set(pos, self.trunk);
                pos += IVec3::Y;
                i += 1;
            }
            if pos.y - origin.y >= 3 {
                ctx.region.set(pos, self.final_large);
                ctx.region.set(pos - IVec3::Y, self.top_large);
                ctx.region.set(pos - IVec3::Y * 2, self.top_small);
            }
        }
        true
    }
}

pub struct Vines {
    pub faces: [BlockId; 5],
}

impl Vines {
    pub fn place(&self, ctx: &mut Context, origin: IVec3) -> bool {
        if !ctx.region.blocks.is_air(ctx.region.get(origin)) {
            return false;
        }
        let faces = [Direction::Up, Direction::North, Direction::South, Direction::West, Direction::East];
        for (direction, vine) in faces.iter().zip(self.faces) {
            if ctx.region.blocks.entry(ctx.region.get(origin + direction.offset())).full {
                ctx.region.set(origin, vine);
                return true;
            }
        }
        false
    }
}

pub struct VegetationPatch {
    pub replaceable: Tag,
    pub ground: StateProvider,
    pub vegetation: Box<Placed>,
    pub ceiling: bool,
    pub depth: IntProvider,
    pub extra_bottom_chance: f32,
    pub vertical_range: i32,
    pub vegetation_chance: f32,
    pub xz_radius: IntProvider,
    pub extra_edge_chance: f32,
    pub water: Option<BlockId>,
}

impl VegetationPatch {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let x_radius = self.xz_radius.sample(random) + 1;
        let z_radius = self.xz_radius.sample(random) + 1;
        let mut surface = self.ground_patch(ctx, random, origin, x_radius, z_radius);
        let inwards = if self.ceiling { IVec3::Y } else { IVec3::NEG_Y };
        if let Some(water) = self.water {
            let mut wet = PositionSet::default();
            for pos in surface.iteration_order() {
                let exposed = [IVec3::NEG_Z, IVec3::X, IVec3::Z, IVec3::NEG_X, IVec3::NEG_Y]
                    .iter()
                    .any(|&side| !ctx.region.blocks.entry(ctx.region.get(pos + side)).full);
                if !exposed {
                    wet.insert(pos);
                }
            }
            for pos in wet.iteration_order() {
                ctx.region.set(pos, water);
            }
            surface = wet;
        }
        for pos in surface.iteration_order() {
            if self.vegetation_chance > 0.0 && random.next_float() < self.vegetation_chance {
                if self.water.is_some() {
                    if self.vegetation.place(ctx, random, pos, None) {
                        let placed = ctx.region.get(pos);
                        let wet = ctx.region.blocks.flooded(placed);
                        ctx.region.set(pos, wet);
                    }
                } else {
                    self.vegetation.place(ctx, random, pos - inwards, None);
                }
            }
        }
        !surface.is_empty()
    }

    pub fn ground_patch(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3, x_radius: i32, z_radius: i32) -> PositionSet {
        let inwards = if self.ceiling { IVec3::Y } else { IVec3::NEG_Y };
        let mut surface = PositionSet::default();
        let blocks = ctx.region.blocks;
        for dx in -x_radius..=x_radius {
            let x_edge = dx == -x_radius || dx == x_radius;
            for dz in -z_radius..=z_radius {
                let z_edge = dz == -z_radius || dz == z_radius;
                let corner = x_edge && z_edge;
                let edge_not_corner = (x_edge || z_edge) && !corner;
                if corner || (edge_not_corner && (self.extra_edge_chance == 0.0 || random.next_float() > self.extra_edge_chance)) {
                    continue;
                }
                let mut pos = origin + IVec3::new(dx, 0, dz);
                let mut offset = 0;
                while blocks.is_air(ctx.region.get(pos)) && offset < self.vertical_range {
                    pos += inwards;
                    offset += 1;
                }
                let mut offset = 0;
                while !blocks.is_air(ctx.region.get(pos)) && offset < self.vertical_range {
                    pos -= inwards;
                    offset += 1;
                }
                let below = pos + inwards;
                if blocks.is_air(ctx.region.get(pos)) && blocks.entry(ctx.region.get(below)).full {
                    let depth = self.depth.sample(random) + if self.extra_bottom_chance > 0.0 && random.next_float() < self.extra_bottom_chance { 1 } else { 0 };
                    if self.place_ground(ctx, random, below, depth, inwards) {
                        surface.insert(below);
                    }
                }
            }
        }
        surface
    }

    fn place_ground(&self, ctx: &mut Context, random: &mut dyn RandomSource, mut pos: IVec3, depth: i32, inwards: IVec3) -> bool {
        let blocks = ctx.region.blocks;
        for i in 0..depth {
            let state = self.ground.state(ctx.region, random, pos);
            let current = ctx.region.get(pos);
            if !blocks.same_kind(state, current) {
                if !blocks.is(current, self.replaceable) {
                    return i != 0;
                }
                ctx.region.set(pos, state);
                pos += inwards;
            }
        }
        true
    }
}

pub fn segmented(table: &mut BlockTable, name: &'static str, min: i32, max: i32) -> Vec<(BlockId, i32)> {
    let mut states = Vec::new();
    for amount in min..=max {
        for direction in Direction::HORIZONTAL {
            let facing = state(MINECRAFT_CARDINAL_DIRECTION, text(direction.name()));
            states.push((table.get(name, &[state(GROWTH, int(amount - 1)), facing]), 1));
        }
    }
    states
}

const DOUBLE_PLANTS: &[&str] = &[SUNFLOWER, LILAC, ROSE_BUSH, PEONY, TALL_GRASS, LARGE_FERN];

fn plant(table: &mut BlockTable, name: &'static str) -> BlockId {
    if DOUBLE_PLANTS.contains(&name) {
        table.get(name, &[state(UPPER_BLOCK_BIT, flag(true))]);
        table.get(name, &[state(UPPER_BLOCK_BIT, flag(false))])
    } else {
        table.name(name)
    }
}

pub fn weighted(table: &mut BlockTable, entries: &[(&'static str, i32)]) -> StateProvider {
    StateProvider::Weighted(entries.iter().map(|&(name, weight)| (plant(table, name), weight)).collect())
}

fn simple_plant(table: &mut BlockTable, name: &'static str) -> Feature {
    let block = plant(table, name);
    simple(block)
}

fn states(table: &mut BlockTable, names: &[&'static str]) -> Vec<BlockId> {
    names.iter().map(|name| plant(table, name)).collect()
}

pub fn pale_moss_patch(table: &mut BlockTable) -> Feature {
    let vegetation = simple(weighted(table, &[(PALE_MOSS_CARPET, 25), (SHORT_GRASS, 25), (TALL_GRASS, 10)]));
    Feature::VegetationPatch(VegetationPatch {
        replaceable: Tag::MossReplaceable,
        ground: table.name(PALE_MOSS_BLOCK).into(),
        vegetation: Box::new(Placed::new(vegetation, Vec::new())),
        ceiling: false,
        depth: 1.into(),
        extra_bottom_chance: 0.0,
        vertical_range: 5,
        vegetation_chance: 0.3,
        xz_radius: IntProvider::Uniform(2, 4),
        extra_edge_chance: 0.75,
        water: None,
    })
}

fn column(table: &mut BlockTable, layers: Vec<(IntProvider, &'static str)>) -> Feature {
    Feature::BlockColumn(BlockColumn {
        layers: layers.into_iter().map(|(height, name)| (height, table.name(name).into())).collect(),
        direction: Direction::Up,
        allowed: empty(),
        prioritize_tip: false,
    })
}

fn bamboo(table: &mut BlockTable, probability: f32) -> Feature {
    let stalk = |table: &mut BlockTable, leaves: &'static str, aged: bool| {
        table.get(
            BAMBOO,
            &[state(BAMBOO_STALK_THICKNESS, text("thick")), state(BAMBOO_LEAF_SIZE, text(leaves)), state(AGE_BIT, flag(aged))],
        )
    };
    Feature::Bamboo(Bamboo {
        probability,
        trunk: stalk(table, "no_leaves", false),
        final_large: stalk(table, "large_leaves", true),
        top_large: stalk(table, "large_leaves", false),
        top_small: stalk(table, "small_leaves", false),
        podzol: table.name(PODZOL),
    })
}

fn noise_provider(first_octave: i32, scale: f32, states: Vec<BlockId>) -> StateProvider {
    StateProvider::Noise {
        noise: NormalNoise::parity(first_octave, &[1.0]).create_legacy(2345),
        scale,
        states,
    }
}

fn patch(tries: i32, xz: i32, y: i32, predicate: BlockPredicate) -> Vec<Modifier> {
    vec![
        count(tries),
        Modifier::Offset(IntProvider::triangle(xz), IntProvider::triangle(y), IntProvider::triangle(xz)),
        filter(predicate),
    ]
}

fn surface(rarity: i32, heightmap: Heightmap) -> Vec<Modifier> {
    let mut modifiers = Vec::new();
    if rarity > 1 {
        modifiers.push(Modifier::Rarity(rarity));
    }
    modifiers.extend([Modifier::InSquare, Modifier::Heightmap(heightmap), Modifier::Biome]);
    modifiers
}

fn with(mut first: Vec<Modifier>, rest: Vec<Modifier>) -> Vec<Modifier> {
    first.extend(rest);
    first
}

fn squared(count_value: i32) -> Vec<Modifier> {
    vec![count(count_value), Modifier::InSquare, Modifier::Heightmap(Heightmap::WorldSurfaceWg), Modifier::Biome]
}

fn mushroom(rarity: i32, prefix: Option<Modifier>) -> Vec<Modifier> {
    let mut modifiers: Vec<Modifier> = prefix.into_iter().collect();
    if rarity != 0 {
        modifiers.push(Modifier::Rarity(rarity));
    }
    modifiers.extend([Modifier::InSquare, Modifier::Heightmap(Heightmap::MotionBlocking), Modifier::Biome]);
    modifiers.extend(patch(96, 7, 3, empty()));
    modifiers
}

fn tree_placement(frequency: Modifier, sapling: Option<BlockId>) -> Vec<Modifier> {
    let mut modifiers = vec![
        frequency,
        Modifier::InSquare,
        Modifier::SurfaceWaterDepth(0),
        Modifier::Heightmap(Heightmap::OceanFloor),
        Modifier::Biome,
    ];
    if let Some(sapling) = sapling {
        modifiers.push(filter(BlockPredicate::WouldSurvive(IVec3::ZERO, sapling)));
    }
    modifiers
}

fn near_water(block: BlockId) -> Modifier {
    let sides = [IVec3::new(1, -1, 0), IVec3::new(-1, -1, 0), IVec3::new(0, -1, 1), IVec3::new(0, -1, -1)];
    let water = BlockPredicate::AnyOf(sides.iter().map(|&offset| BlockPredicate::MatchingFluids(offset, vec![Fluid::Water])).collect());
    filter(BlockPredicate::AllOf(vec![empty(), BlockPredicate::WouldSurvive(IVec3::ZERO, block), water]))
}

fn on_grass(table: &mut BlockTable) -> BlockPredicate {
    BlockPredicate::AllOf(vec![empty(), BlockPredicate::MatchingBlocks(IVec3::NEG_Y, vec![table.kind(GRASS_BLOCK)])])
}

fn noise_threshold(level: f64, below: i32, above: i32) -> Modifier {
    Modifier::NoiseThresholdCount { level, below, above }
}

fn random_selector(features: Vec<(Placed, f32)>, default: Placed) -> Feature {
    Feature::RandomSelector { features, default: Box::new(default) }
}

pub fn vines(table: &mut BlockTable) -> Feature {
    Feature::Vines(Vines {
        faces: [Direction::Up, Direction::North, Direction::South, Direction::West, Direction::East].map(|face| table.get(VINE, &[state(VINE_DIRECTION_BITS, int(vine_bit(face)))])),
    })
}

fn segmented_patch(table: &mut BlockTable, name: &'static str, max: i32) -> Feature {
    simple(StateProvider::Weighted(segmented(table, name, 1, max)))
}

fn flower_forest_flowers(table: &mut BlockTable) -> Feature {
    let flowers = states(
        table,
        &[
            DANDELION,
            POPPY,
            ALLIUM,
            AZURE_BLUET,
            RED_TULIP,
            ORANGE_TULIP,
            WHITE_TULIP,
            PINK_TULIP,
            OXEYE_DAISY,
            CORNFLOWER,
            LILY_OF_THE_VALLEY,
        ],
    );
    simple(noise_provider(0, 0.020833334, flowers))
}

fn meadow_flowers(table: &mut BlockTable) -> Feature {
    let flowers = states(table, &[TALL_GRASS, ALLIUM, POPPY, AZURE_BLUET, DANDELION, CORNFLOWER, OXEYE_DAISY, SHORT_GRASS]);
    simple(StateProvider::DualNoise {
        noise: NormalNoise::parity(-3, &[1.0]).create_legacy(2345),
        slow_noise: NormalNoise::parity(-10, &[1.0]).create_legacy(2345),
        scale: 1.0,
        slow_scale: 1.0,
        variety: (1, 3),
        states: flowers,
    })
}

fn plains_flowers(table: &mut BlockTable) -> Feature {
    let low = states(table, &[ORANGE_TULIP, RED_TULIP, PINK_TULIP, WHITE_TULIP]);
    let high = states(table, &[POPPY, AZURE_BLUET, OXEYE_DAISY, CORNFLOWER]);
    simple(StateProvider::NoiseThreshold {
        noise: NormalNoise::parity(0, &[1.0]).create_legacy(2345),
        scale: 0.005,
        threshold: -0.8,
        high_chance: 0.33333334,
        default: table.name(DANDELION),
        low,
        high,
    })
}

fn forest_flowers(table: &mut BlockTable) -> Feature {
    let features = [LILAC, ROSE_BUSH, PEONY, LILY_OF_THE_VALLEY]
        .iter()
        .map(|name| Placed::new(simple_plant(table, name), patch(96, 7, 3, empty())))
        .collect();
    Feature::SimpleRandomSelector(features)
}

fn dark_forest_trees(table: &mut BlockTable) -> Feature {
    random_selector(
        vec![
            (trees::inline(table, TreeFeature::HugeBrownMushroom), 0.025),
            (trees::inline(table, TreeFeature::HugeRedMushroom), 0.05),
            (trees::checked(table, Species::DarkOak.with_leaf_litter()), 0.6666667),
            (trees::checked(table, Fallen::Birch), 0.0025),
            (trees::checked(table, Species::Birch.with_leaf_litter()), 0.2),
            (trees::checked(table, Fallen::Oak), 0.0125),
            (trees::checked(table, Species::FancyOak.with_leaf_litter()), 0.1),
        ],
        trees::checked(table, Species::Oak.with_leaf_litter()),
    )
}

fn dappled_forest_trees(table: &mut BlockTable) -> Feature {
    Feature::WeightedSelector(vec![
        (trees::checked(table, Species::RedPoplar.with_leaf_litter()), 200),
        (trees::checked(table, Species::OrangePoplar.with_leaf_litter()), 240),
        (trees::checked(table, Species::YellowPoplar.with_leaf_litter()), 90),
        (trees::checked(table, Species::Spruce), 27),
        (trees::checked(table, Fallen::Poplar), 120),
    ])
}

fn savanna_trees(table: &mut BlockTable) -> Feature {
    random_selector(
        vec![(trees::checked(table, Species::Acacia), 0.8), (trees::checked(table, Fallen::Oak), 0.0125)],
        trees::checked(table, Species::Oak),
    )
}

fn windswept_hills_trees(table: &mut BlockTable) -> Feature {
    random_selector(
        vec![
            (trees::checked(table, Fallen::Spruce), 0.008325),
            (trees::checked(table, Species::Spruce), 0.666),
            (trees::checked(table, Species::FancyOak), 0.1),
            (trees::checked(table, Fallen::Oak), 0.0125),
        ],
        trees::checked(table, Species::Oak),
    )
}

fn old_growth_taiga_trees(table: &mut BlockTable, pine: bool) -> Feature {
    let mut features = if pine {
        vec![(trees::checked(table, Species::MegaSpruce), 0.025641026), (trees::checked(table, Species::MegaPine), 0.30769232)]
    } else {
        vec![(trees::checked(table, Species::MegaSpruce), 0.33333334)]
    };
    features.push((trees::checked(table, Species::Pine), 0.33333334));
    features.push((trees::checked(table, Fallen::Spruce), 0.0125));
    random_selector(features, trees::checked(table, Species::Spruce))
}

fn bamboo_jungle_vegetation(table: &mut BlockTable) -> Feature {
    let features = vec![
        (trees::checked(table, Species::FancyOak), 0.05),
        (trees::checked(table, Species::JungleBush), 0.15),
        (trees::checked(table, Species::MegaJungle), 0.7),
    ];
    let podzol = table.kind(PODZOL);
    let grass = simple(weighted(table, &[(SHORT_GRASS, 3), (FERN, 1)]));
    let not_on_podzol = BlockPredicate::AllOf(vec![empty(), BlockPredicate::MatchingBlocks(IVec3::NEG_Y, vec![podzol]).not()]);
    random_selector(features, Placed::new(grass, patch(32, 7, 3, not_on_podzol)))
}

pub fn define(key: Key, table: &mut BlockTable) -> Option<Placed> {
    use Heightmap::{MotionBlocking, MotionBlockingNoLeaves, OceanFloor, WorldSurfaceWg};
    let tufted = |tries: i32| {
        with(
            vec![noise_threshold(-0.8, 5, 10), Modifier::InSquare, Modifier::Heightmap(WorldSurfaceWg), Modifier::Biome],
            patch(tries, 7, 3, empty()),
        )
    };
    let flowering = |tries: i32| {
        with(
            vec![noise_threshold(-0.8, 5, 10), Modifier::InSquare, Modifier::Heightmap(MotionBlocking), Modifier::Biome],
            patch(tries, 6, 2, empty()),
        )
    };
    let (feature, placement) = match key {
        Key::BambooLight => (bamboo(table, 0.0), surface(4, MotionBlocking)),
        Key::Bamboo => (
            bamboo(table, 0.2),
            vec![
                Modifier::NoiseBasedCount {
                    ratio: 160,
                    factor: 80.0,
                    offset: 0.3,
                },
                Modifier::InSquare,
                Modifier::Heightmap(WorldSurfaceWg),
                Modifier::Biome,
            ],
        ),
        Key::Vines => (
            vines(table),
            vec![
                count(127),
                Modifier::InSquare,
                Modifier::HeightRange(super::provider::HeightProvider::Uniform(super::provider::Anchor::Absolute(64), super::provider::Anchor::Absolute(100))),
                Modifier::Biome,
            ],
        ),
        Key::PatchSunflower => (simple_plant(table, SUNFLOWER), with(surface(3, MotionBlocking), patch(96, 7, 3, empty()))),
        Key::PatchPumpkin => (simple_plant(table, PUMPKIN), with(surface(300, MotionBlocking), patch(96, 7, 3, on_grass(table)))),
        Key::PatchGrassPlain => (simple_plant(table, SHORT_GRASS), tufted(32)),
        Key::PatchGrassMeadow => (simple_plant(table, SHORT_GRASS), tufted(16)),
        Key::PatchGrassForest => (simple_plant(table, SHORT_GRASS), with(squared(2), patch(32, 7, 3, empty()))),
        Key::PatchLeafLitter => (segmented_patch(table, LEAF_LITTER, 3), with(squared(2), patch(32, 7, 3, on_grass(table)))),
        Key::PatchGrassBadlands => (simple_plant(table, SHORT_GRASS), with(surface(1, WorldSurfaceWg), patch(32, 7, 3, empty()))),
        Key::PatchGrassSavanna => (simple_plant(table, SHORT_GRASS), with(squared(20), patch(32, 7, 3, empty()))),
        Key::PatchGrassNormal => (simple_plant(table, SHORT_GRASS), with(squared(5), patch(32, 7, 3, empty()))),
        Key::PatchGrassTaiga2 => (simple(weighted(table, &[(SHORT_GRASS, 1), (FERN, 4)])), with(surface(1, WorldSurfaceWg), patch(32, 7, 3, empty()))),
        Key::PatchGrassTaiga => (simple(weighted(table, &[(SHORT_GRASS, 1), (FERN, 4)])), with(squared(7), patch(32, 7, 3, empty()))),
        Key::PatchGrassJungle => {
            let podzol = table.kind(PODZOL);
            let predicate = BlockPredicate::AllOf(vec![empty(), BlockPredicate::MatchingBlocks(IVec3::NEG_Y, vec![podzol]).not()]);
            (simple(weighted(table, &[(SHORT_GRASS, 3), (FERN, 1)])), with(squared(25), patch(32, 7, 3, predicate)))
        }
        Key::PatchDeadBush2 => (simple_plant(table, DEADBUSH), with(squared(2), patch(4, 7, 3, empty()))),
        Key::PatchDeadBush => (simple_plant(table, DEADBUSH), with(surface(1, WorldSurfaceWg), patch(4, 7, 3, empty()))),
        Key::PatchDeadBushBadlands => (simple_plant(table, DEADBUSH), with(squared(20), patch(4, 7, 3, empty()))),
        Key::PatchDryGrassBadlands | Key::PatchDryGrassDesert => {
            let rarity = if key == Key::PatchDryGrassBadlands { 6 } else { 3 };
            (
                simple(weighted(table, &[(SHORT_DRY_GRASS, 1), (TALL_DRY_GRASS, 1)])),
                with(surface(rarity, MotionBlocking), patch(64, 7, 3, empty())),
            )
        }
        Key::PatchMelon | Key::PatchMelonSparse => {
            let grass = table.kind(GRASS_BLOCK);
            let predicate = BlockPredicate::AllOf(vec![
                BlockPredicate::Replaceable(IVec3::ZERO),
                BlockPredicate::NoFluid(IVec3::ZERO),
                BlockPredicate::MatchingBlocks(IVec3::NEG_Y, vec![grass]),
            ]);
            let rarity = if key == Key::PatchMelon { 6 } else { 64 };
            (simple_plant(table, MELON_BLOCK), with(surface(rarity, MotionBlocking), patch(64, 7, 3, predicate)))
        }
        Key::PatchBerryCommon | Key::PatchBerryRare => {
            let rarity = if key == Key::PatchBerryCommon { 32 } else { 384 };
            (
                simple(table.get(SWEET_BERRY_BUSH, &[state(GROWTH, int(3))])),
                with(surface(rarity, WorldSurfaceWg), patch(96, 7, 3, on_grass(table))),
            )
        }
        Key::PatchWaterlily => (simple_plant(table, WATERLILY), with(squared(4), patch(10, 7, 3, empty()))),
        Key::PatchTallGrass2 => (
            simple_plant(table, TALL_GRASS),
            with(with(vec![noise_threshold(-0.8, 0, 7)], surface(32, MotionBlocking)), patch(96, 7, 3, empty())),
        ),
        Key::PatchTallGrass => (simple_plant(table, TALL_GRASS), with(surface(5, MotionBlocking), patch(96, 7, 3, empty()))),
        Key::PatchLargeFern => (simple_plant(table, LARGE_FERN), with(surface(5, MotionBlocking), patch(96, 7, 3, empty()))),
        Key::PatchBush => (simple_plant(table, BUSH), with(surface(4, MotionBlocking), patch(24, 5, 3, empty()))),
        Key::PatchRedShrub => (simple_plant(table, RED_SHRUB), with(surface(1, WorldSurfaceWg), patch(8, 7, 3, empty()))),
        Key::PatchCactusDesert | Key::PatchCactusDecorated => {
            let cactus = table.name(CACTUS);
            let predicate = BlockPredicate::AllOf(vec![empty(), BlockPredicate::WouldSurvive(IVec3::ZERO, cactus)]);
            let rarity = if key == Key::PatchCactusDesert { 6 } else { 13 };
            let layers = vec![(IntProvider::BiasedToBottom(1, 3), CACTUS), (IntProvider::Weighted(vec![(0.into(), 3), (1.into(), 1)]), CACTUS_FLOWER)];
            (column(table, layers), with(surface(rarity, MotionBlocking), patch(10, 7, 3, predicate)))
        }
        Key::PatchSugarCaneSwamp | Key::PatchSugarCaneDesert | Key::PatchSugarCaneBadlands | Key::PatchSugarCane => {
            let rarity = match key {
                Key::PatchSugarCaneSwamp => 3,
                Key::PatchSugarCaneDesert => 1,
                Key::PatchSugarCaneBadlands => 5,
                _ => 6,
            };
            let cane = table.name(REEDS);
            let offset = Modifier::Offset(IntProvider::triangle(4), IntProvider::triangle(0), IntProvider::triangle(4));
            (
                column(table, vec![(IntProvider::BiasedToBottom(2, 4), REEDS)]),
                with(surface(rarity, MotionBlocking), vec![count(20), offset, near_water(cane)]),
            )
        }
        Key::PatchFireflyBushNearWater | Key::PatchFireflyBushNearWaterSwamp => {
            let bush = table.name(FIREFLY_BUSH);
            let (tries, heightmap) = if key == Key::PatchFireflyBushNearWater {
                (2, MotionBlockingNoLeaves)
            } else {
                (3, MotionBlocking)
            };
            let base = vec![count(tries), Modifier::InSquare, Modifier::Heightmap(heightmap), Modifier::Biome, near_water(bush)];
            (simple_plant(table, FIREFLY_BUSH), with(base, patch(20, 4, 3, empty())))
        }
        Key::PatchFireflyBushSwamp => (simple_plant(table, FIREFLY_BUSH), with(surface(8, MotionBlocking), patch(20, 4, 3, empty()))),
        Key::BrownMushroomNormal => (simple_plant(table, BROWN_MUSHROOM), mushroom(256, None)),
        Key::RedMushroomNormal => (simple_plant(table, RED_MUSHROOM), mushroom(512, None)),
        Key::BrownMushroomTaiga => (simple_plant(table, BROWN_MUSHROOM), mushroom(4, None)),
        Key::RedMushroomTaiga => (simple_plant(table, RED_MUSHROOM), mushroom(256, None)),
        Key::BrownMushroomOldGrowth => (simple_plant(table, BROWN_MUSHROOM), mushroom(4, Some(count(3)))),
        Key::RedMushroomOldGrowth => (simple_plant(table, RED_MUSHROOM), mushroom(171, None)),
        Key::BrownMushroomSwamp => (simple_plant(table, BROWN_MUSHROOM), mushroom(0, Some(count(2)))),
        Key::RedMushroomSwamp => (simple_plant(table, RED_MUSHROOM), mushroom(64, None)),
        Key::BrownMushroomDappledForest => (simple_plant(table, BROWN_MUSHROOM), mushroom(2, None)),
        Key::FlowerWarm | Key::FlowerDefault => {
            let rarity = if key == Key::FlowerWarm { 16 } else { 32 };
            (simple(weighted(table, &[(POPPY, 2), (DANDELION, 1)])), with(surface(rarity, MotionBlocking), patch(64, 7, 3, empty())))
        }
        Key::FlowerFlowerForest => (flower_forest_flowers(table), with(with(vec![count(3)], surface(2, MotionBlocking)), patch(96, 6, 2, empty()))),
        Key::FlowerSwamp => (simple_plant(table, BLUE_ORCHID), with(surface(32, MotionBlocking), patch(64, 6, 2, empty()))),
        Key::FlowerPlains => (
            plains_flowers(table),
            with(with(vec![noise_threshold(-0.8, 15, 4)], surface(32, MotionBlocking)), patch(64, 6, 2, empty())),
        ),
        Key::FlowerCherry => (segmented_patch(table, PINK_PETALS, 4), flowering(96)),
        Key::FlowerMeadow => (meadow_flowers(table), with(surface(1, MotionBlocking), patch(96, 6, 2, empty()))),
        Key::FlowerPaleGarden => (simple_plant(table, CLOSED_EYEBLOSSOM), surface(32, MotionBlocking)),
        Key::WildflowersBirchForest => (segmented_patch(table, WILDFLOWERS, 4), with(with(vec![count(3)], surface(2, MotionBlocking)), patch(64, 6, 2, empty()))),
        Key::WildflowersMeadow => (segmented_patch(table, WILDFLOWERS, 4), flowering(8)),
        Key::TreesPlains => {
            let sapling = table.name(OAK_SAPLING);
            let placement = vec![
                count_extra(0, 0.05, 1),
                Modifier::InSquare,
                Modifier::SurfaceWaterDepth(0),
                Modifier::Heightmap(OceanFloor),
                filter(BlockPredicate::WouldSurvive(IVec3::ZERO, sapling)),
                Modifier::Biome,
            ];
            let feature = random_selector(
                vec![(trees::inline(table, Species::FancyOak.with_bees(0.05)), 0.33333334), (trees::checked(table, Fallen::Oak), 0.0125)],
                trees::inline(table, Species::Oak.with_bees(0.05)),
            );
            (feature, placement)
        }
        Key::DarkForestVegetation => (dark_forest_trees(table), tree_placement(count(16), None)),
        Key::TreesDappledForest => (dappled_forest_trees(table), tree_placement(count(6), None)),
        Key::PaleGardenVegetation => {
            let feature = random_selector(
                vec![(trees::checked(table, Species::CreakingPaleOak), 0.1), (trees::checked(table, Species::PaleOak), 0.9)],
                trees::checked(table, Species::PaleOak),
            );
            (feature, tree_placement(count(16), None))
        }
        Key::FlowerForestFlowers | Key::ForestFlowers => {
            let range = if key == Key::FlowerForestFlowers { (-1, 3, 3) } else { (-3, 1, 1) };
            let counted = Modifier::Count(IntProvider::Clamped(Box::new(IntProvider::Uniform(range.0, range.1)), 0, range.2));
            (
                forest_flowers(table),
                vec![Modifier::Rarity(7), Modifier::InSquare, Modifier::Heightmap(MotionBlocking), counted, Modifier::Biome],
            )
        }
        Key::PaleGardenFlowers => (simple_plant(table, CLOSED_EYEBLOSSOM), with(surface(8, MotionBlockingNoLeaves), patch(96, 7, 3, empty()))),
        Key::PaleMossPatch => (pale_moss_patch(table), vec![count(1), Modifier::InSquare, Modifier::Heightmap(MotionBlockingNoLeaves), Modifier::Biome]),
        Key::TreesFlowerForest => {
            let feature = random_selector(
                vec![
                    (trees::checked(table, Fallen::Birch), 0.0025),
                    (trees::checked(table, Species::Birch.with_bees(0.02)), 0.2),
                    (trees::checked(table, Species::FancyOak.with_bees(0.02)), 0.1),
                ],
                trees::checked(table, Species::Oak.with_bees(0.02)),
            );
            (feature, tree_placement(count_extra(6, 0.1, 1), None))
        }
        Key::TreesMeadow => {
            let feature = random_selector(
                vec![(trees::checked(table, Species::FancyOak.with_bees(1.0)), 0.5)],
                trees::checked(table, Species::SuperBirch.with_bees(1.0)),
            );
            (feature, tree_placement(Modifier::Rarity(100), None))
        }
        Key::TreesCherry => {
            let sapling = table.name(CHERRY_SAPLING);
            (
                TreeFeature::from(Species::Cherry.with_bees(0.05)).feature(table),
                tree_placement(count_extra(10, 0.1, 1), Some(sapling)),
            )
        }
        Key::TreesTaiga => {
            let feature = random_selector(
                vec![(trees::checked(table, Species::Pine), 0.33333334), (trees::checked(table, Fallen::Spruce), 0.0125)],
                trees::checked(table, Species::Spruce),
            );
            (feature, tree_placement(count_extra(10, 0.1, 1), None))
        }
        Key::TreesGrove => {
            let feature = random_selector(vec![(trees::on_snow(table, Species::Pine), 0.33333334)], trees::on_snow(table, Species::Spruce));
            (feature, tree_placement(count_extra(10, 0.1, 1), None))
        }
        Key::TreesBadlands => {
            let sapling = table.name(OAK_SAPLING);
            let feature = random_selector(vec![(trees::checked(table, Fallen::Oak), 0.0125)], trees::checked(table, Species::Oak.with_leaf_litter()));
            (feature, tree_placement(count_extra(5, 0.1, 1), Some(sapling)))
        }
        Key::TreesSnowy => {
            let sapling = table.name(SPRUCE_SAPLING);
            let feature = random_selector(vec![(trees::checked(table, Fallen::Spruce), 0.0125)], trees::checked(table, Species::Spruce));
            (feature, tree_placement(count_extra(0, 0.1, 1), Some(sapling)))
        }
        Key::TreesSwamp => {
            let sapling = table.name(OAK_SAPLING);
            let placement = vec![
                count_extra(2, 0.1, 1),
                Modifier::InSquare,
                Modifier::SurfaceWaterDepth(2),
                Modifier::Heightmap(OceanFloor),
                Modifier::Biome,
                filter(BlockPredicate::WouldSurvive(IVec3::ZERO, sapling)),
            ];
            (TreeFeature::from(Species::SwampOak).feature(table), placement)
        }
        Key::TreesWindsweptSavanna => (savanna_trees(table), tree_placement(count_extra(2, 0.1, 1), None)),
        Key::TreesSavanna => (savanna_trees(table), tree_placement(count_extra(1, 0.1, 1), None)),
        Key::BirchTall => {
            let feature = random_selector(
                vec![
                    (trees::checked(table, Fallen::SuperBirch), 0.00625),
                    (trees::checked(table, Species::SuperBirch.with_bees(0.002)), 0.5),
                    (trees::checked(table, Fallen::Birch), 0.0125),
                ],
                trees::checked(table, Species::Birch.with_bees(0.002)),
            );
            (feature, tree_placement(count_extra(10, 0.1, 1), None))
        }
        Key::TreesBirch => {
            let sapling = table.name(BIRCH_SAPLING);
            let feature = random_selector(vec![(trees::checked(table, Fallen::Birch), 0.0125)], trees::checked(table, Species::Birch.with_bees(0.002)));
            (feature, tree_placement(count_extra(10, 0.1, 1), Some(sapling)))
        }
        Key::TreesWindsweptForest => (windswept_hills_trees(table), tree_placement(count_extra(3, 0.1, 1), None)),
        Key::TreesWindsweptHills => (windswept_hills_trees(table), tree_placement(count_extra(0, 0.1, 1), None)),
        Key::TreesWater => {
            let feature = random_selector(vec![(trees::checked(table, Species::FancyOak), 0.1)], trees::checked(table, Species::Oak));
            (feature, tree_placement(count_extra(0, 0.1, 1), None))
        }
        Key::TreesBirchAndOakLeafLitter => {
            let feature = random_selector(
                vec![
                    (trees::checked(table, Fallen::Birch), 0.0025),
                    (trees::checked(table, Species::Birch.with_bees(0.002).with_leaf_litter()), 0.2),
                    (trees::checked(table, Species::FancyOak.with_bees(0.002).with_leaf_litter()), 0.1),
                    (trees::checked(table, Fallen::Oak), 0.0125),
                ],
                trees::checked(table, Species::Oak.with_bees(0.002).with_leaf_litter()),
            );
            (feature, tree_placement(count_extra(10, 0.1, 1), None))
        }
        Key::TreesSparseJungle | Key::TreesJungle => {
            let mut features = vec![(trees::checked(table, Species::FancyOak), 0.1), (trees::checked(table, Species::JungleBush), 0.5)];
            if key == Key::TreesJungle {
                features.push((trees::checked(table, Species::MegaJungle), 0.33333334));
            }
            features.push((trees::checked(table, Fallen::Jungle), 0.0125));
            let frequency = if key == Key::TreesJungle { count_extra(50, 0.1, 1) } else { count_extra(2, 0.1, 1) };
            (random_selector(features, trees::checked(table, Species::Jungle)), tree_placement(frequency, None))
        }
        Key::TreesOldGrowthSpruceTaiga => (old_growth_taiga_trees(table, false), tree_placement(count_extra(10, 0.1, 1), None)),
        Key::TreesOldGrowthPineTaiga => (old_growth_taiga_trees(table, true), tree_placement(count_extra(10, 0.1, 1), None)),
        Key::BambooVegetation => (bamboo_jungle_vegetation(table), tree_placement(count_extra(30, 0.1, 1), None)),
        Key::MushroomIslandVegetation => (
            Feature::RandomBoolean(
                Box::new(trees::inline(table, TreeFeature::HugeRedMushroom)),
                Box::new(trees::inline(table, TreeFeature::HugeBrownMushroom)),
            ),
            surface(1, MotionBlocking),
        ),
        Key::TreesMangrove => (
            random_selector(vec![(trees::checked(table, Species::TallMangrove), 0.85)], trees::checked(table, Species::Mangrove)),
            vec![count(25), Modifier::InSquare, Modifier::SurfaceWaterDepth(5), Modifier::Heightmap(OceanFloor), Modifier::Biome],
        ),
        _ => return None,
    };
    Some(Placed::new(feature, placement))
}
