use super::expression::{Expression, Host, Variables};
use super::ore::{Cells, Draw, Ore, ReplaceRule};
use super::registry::FeatureRegistry;
use super::rules::{self, Feature, Loaded, load_with_registry};
use super::scatter::{Distribution, Draws as ScatterDraws, Scatter};
use super::seed::{decoration_seed, large_feature_multipliers, rule_seed};
use crate::shared::biome::Biome;
use crate::shared::blocks::{AIR, BlockId, BlockTable};
use crate::shared::hash::fnv1_64;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_registry::BlockRegistry;
use glam::IVec3;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::LazyLock;

const GENERAL_1: i64 = 4802433367836389877;

pub(super) struct NoHost;

impl Host for NoHost {
    fn heightmap(&mut self, _x: i32, _z: i32) -> i32 {
        0
    }

    fn above_top_solid(&mut self, _x: i32, _z: i32) -> i32 {
        0
    }
}

pub(super) fn load(table: &mut BlockTable) -> Loaded {
    load_with(table, &[])
}

pub(super) fn load_with(table: &mut BlockTable, extra_features: &[Arc<str>]) -> Loaded {
    load_with_registry(table, extra_features, &FeatureRegistry::vanilla())
}

const STONE_CELL: BlockId = 1;
const ORE_CELL: BlockId = 2;

#[test]
fn rule_identifier_hash_matches_the_binary() {
    assert_eq!(fnv1_64("variable.is_legacy"), 0x7998477002c44401);
}

#[test]
fn multipliers_are_successive_odd_next_ints() {
    let level_seed = GENERAL_1 as u32;
    let mut random = MersenneTwister::new(level_seed);
    let expected = ((random.next_u32() >> 1) | 1, (random.next_u32() >> 1) | 1);
    assert_eq!(large_feature_multipliers(level_seed), expected);
    assert_ne!(expected.0, expected.1);
    assert!(expected.0 & 1 == 1 && expected.1 & 1 == 1);
    assert!(expected.0 < 1 << 31 && expected.1 < 1 << 31);
}

#[test]
fn decoration_seed_combines_the_chunk_coordinates() {
    let level_seed = GENERAL_1 as u32;
    let (x_multiplier, z_multiplier) = large_feature_multipliers(level_seed);
    assert_eq!(decoration_seed(level_seed, 0, 0), level_seed);
    assert_eq!(decoration_seed(level_seed, 3, 0), level_seed ^ 3u32.wrapping_mul(x_multiplier));
    assert_eq!(decoration_seed(level_seed, 0, -2), level_seed ^ (-2i32 as u32).wrapping_mul(z_multiplier));
    assert_eq!(
        decoration_seed(level_seed, -5, 7),
        level_seed ^ 7u32.wrapping_mul(z_multiplier).wrapping_add((-5i32 as u32).wrapping_mul(x_multiplier))
    );
}

#[test]
fn rule_seed_follows_the_hash_combine_formula() {
    assert_eq!(rule_seed(0x7998477002c44401, 1), 0xA0FBBDFB);
    assert_eq!(rule_seed(0x7998477002c44401, 0), 0x02c44401u32.wrapping_add(0x9E3779B9));
}

fn replay(seed: u32) -> MersenneTwister {
    MersenneTwister::new(seed)
}

fn sample(distribution: Distribution, min: i32, max: i32, step: i32, offset: i32, index: i32) -> (i32, i32) {
    let mut random = replay(9);
    let mut grid_index = index;
    (distribution.sample(&mut random, min, max, step, offset, &mut grid_index), grid_index)
}

#[test]
fn single_valued_draws_nothing() {
    assert_eq!(sample(Distribution::SingleValued, 5, 20, 1, 0, 0).0, 5);
}

#[test]
fn uniform_is_a_modulo_draw_with_an_exclusive_bound() {
    assert_eq!(sample(Distribution::Uniform, -4, 12, 1, 0, 0).0, -4 + (replay(9).next_u32() % 16) as i32);
    assert_eq!(sample(Distribution::Uniform, 7, 7, 1, 0, 0).0, 7);
}

#[test]
fn gaussian_subtracts_two_modulo_draws() {
    let mut words = replay(9);
    let half = 5u32;
    let offset = (words.next_u32() % half) as i32 - (words.next_u32() % half) as i32;
    assert_eq!(sample(Distribution::Gaussian, 0, 10, 1, 0, 0).0, 5 + offset);
    assert_eq!(sample(Distribution::Gaussian, 0, 0, 1, 0, 0).0, 0);
}

#[test]
fn inverse_gaussian_mirrors_the_offset_to_the_edges() {
    let mut words = replay(9);
    let half = 8u32;
    let offset = (words.next_u32() % half) as i32 - (words.next_u32() % half) as i32;
    let expected = match offset {
        0 if words.next_boolean() => 16,
        0 => 0,
        offset if offset < 0 => -offset,
        offset => 16 - offset,
    };
    assert_eq!(sample(Distribution::InverseGaussian, 0, 16, 1, 0, 0).0, expected);
}

#[test]
fn grids_walk_by_the_step_and_carry_the_index() {
    assert_eq!(sample(Distribution::FixedGrid, 0, 15, 4, 1, 3), (13, 13 / 16));
    assert_eq!(sample(Distribution::FixedGrid, 0, 15, 4, 0, 5), (4, 20 / 16));
    let mut words = replay(9);
    let jitter = (words.next_u32() % 4) as i32;
    assert_eq!(sample(Distribution::JitteredGrid, 0, 15, 4, 0, 2).0, 8 + jitter);
}

#[test]
fn triangle_adds_two_half_range_draws() {
    let mut words = replay(9);
    let first = (words.next_u32() % 49) as i32;
    let second = (words.next_u32() % 49) as i32;
    assert_eq!(sample(Distribution::Triangle, -10, 86, 1, 0, 0).0, -10 + first + second);
}

fn evaluate(text: &str) -> f32 {
    let variables = Variables {
        origin: [32.0, 64.0, -16.0],
        world: [1.0, 2.0, 3.0],
    };
    Expression::parse(text).evaluate(&variables, &mut NoHost, &mut MersenneTwister::new(7))
}

#[test]
fn expressions_cover_what_the_rules_use() {
    assert_eq!(evaluate("17"), 17.0);
    assert_eq!(evaluate("variable.worldx + variable.worldz * 2"), 7.0);
    assert_eq!(evaluate("variable.is_legacy ? 0.0 : 100.0"), 100.0);
    assert_eq!(evaluate("variable.is_legacy == 0.0 ? 1 : 0"), 1.0);
    assert_eq!(evaluate("(variable.originx + 8) / 4"), 10.0);
    assert_eq!(evaluate("variable.originy"), 64.0);
    assert_eq!(evaluate("math.ceil(2.1) + math.max(1, 5) - math.min(4, 2)"), 6.0);
    assert_eq!(evaluate("(1 < 2) && variable.is_legacy == 0.0 ? 7 : 0"), 7.0);
    assert_eq!(evaluate("-variable.worldy"), -2.0);
}

#[test]
fn math_random_draws_one_float_from_the_generator() {
    let mut words = MersenneTwister::new(7);
    let expected = 48.0 + (96.0 - 48.0) * words.next_float();
    assert_eq!(evaluate("Math.Random(48, 96)"), expected);
    assert_eq!(evaluate("math.random(-8, 8)"), -8.0 + 16.0 * MersenneTwister::new(7).next_float());
}

#[test]
fn math_random_keeps_the_generator_mapping_not_the_crate_sample() {
    let variables = Variables {
        origin: [32.0, 64.0, -16.0],
        world: [1.0, 2.0, 3.0],
    };
    let mut source = MersenneTwister::new(3);
    let word = source.next_u32();
    assert!(word >> 31 == 1);
    let drawn = Expression::parse("math.random(0, 1)").evaluate(&variables, &mut NoHost, &mut MersenneTwister::new(3));
    assert_eq!(drawn, MersenneTwister::new(3).next_float());
    assert_eq!(drawn, MersenneTwister::float_from_word(word));
    assert_ne!(drawn, molangx::rng::sample(&mut molangx::rng::FixedRng(word)));
}

#[test]
fn queries_without_a_host_answer_zero_and_noise_is_the_info_octave() {
    assert_eq!(evaluate("query.heightmap(variable.worldx, variable.worldz)"), 0.0);
    assert_eq!(evaluate("query.above_top_solid(1, 2)"), 0.0);
    assert_eq!(evaluate("query.noise(1.5, 2.5)"), crate::bedrock::overworld::surface::info_noise(1.5, 2.5));
    assert_ne!(evaluate("query.noise(1.5, 2.5)"), 0.0);
    assert_eq!(evaluate("query.unknown(1)"), 0.0);
    let mut found = BTreeMap::new();
    Expression::parse("math.max(query.unknown(variable.worldx, 1), 2)").collect_unsupported(&mut found);
    assert_eq!(found.keys().collect::<Vec<_>>(), ["query.unknown"]);
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Draws {
    Float,
    Below(i32),
    Word,
    Double,
    Boolean,
}

struct Recorder {
    random: MersenneTwister,
    log: Vec<Draws>,
}

impl Recorder {
    fn new(seed: u32) -> Self {
        Self {
            random: MersenneTwister::new(seed),
            log: Vec::new(),
        }
    }
}

impl Draw for Recorder {
    fn float(&mut self) -> f32 {
        self.log.push(Draws::Float);
        self.random.next_float()
    }

    fn below(&mut self, bound: i32) -> i32 {
        self.log.push(Draws::Below(bound));
        self.random.next_int_bounded(bound)
    }

    fn word(&mut self) -> u32 {
        self.log.push(Draws::Word);
        self.random.next_u32()
    }

    fn double(&mut self) -> f64 {
        self.log.push(Draws::Double);
        self.random.next_double()
    }

    fn boolean(&mut self) -> bool {
        self.log.push(Draws::Boolean);
        self.random.next_boolean()
    }
}

struct Grid {
    cells: HashMap<IVec3, BlockId>,
}

impl Grid {
    fn block_of_stone(size: IVec3, corner: IVec3) -> Self {
        let cells = (0..size.x)
            .flat_map(|x| (0..size.y).flat_map(move |y| (0..size.z).map(move |z| corner + IVec3::new(x, y, z))))
            .map(|pos| (pos, STONE_CELL))
            .collect();
        Self { cells }
    }

    fn count(&self, block: BlockId) -> usize {
        self.cells.values().filter(|&&found| found == block).count()
    }
}

impl Cells for Grid {
    fn block(&self, pos: IVec3) -> BlockId {
        self.cells.get(&pos).copied().unwrap_or(AIR)
    }

    fn place(&mut self, pos: IVec3, block: BlockId) {
        self.cells.insert(pos, block);
    }

    fn kind(&self, block: BlockId) -> u16 {
        block
    }

    fn is_air(&self, block: BlockId) -> bool {
        block == AIR
    }

    fn min_y(&self) -> i32 {
        -64
    }

    fn max_y(&self) -> i32 {
        319
    }
}

fn ore(count: i32, discard_chance: f32) -> Ore {
    Ore {
        count,
        discard_chance,
        rules: vec![ReplaceRule {
            block: ORE_CELL,
            may_replace: vec![STONE_CELL],
        }],
    }
}

fn buried_world() -> Grid {
    Grid::block_of_stone(IVec3::new(48, 24, 48), IVec3::new(-8, -12, -8))
}

#[test]
fn ore_draws_follow_the_captured_order() {
    for count in [9, 17, 33, 64] {
        let mut world = buried_world();
        let mut recorder = Recorder::new(77);
        let placed = ore(count, 0.0).place(&mut world, &mut recorder, IVec3::new(0, 0, 0));
        let mut expected = vec![Draws::Float, Draws::Below(3), Draws::Below(3)];
        expected.extend(std::iter::repeat_n(Draws::Float, count as usize));
        assert_eq!(recorder.log, expected, "count {count}");
        assert!(placed);
        assert!(world.count(ORE_CELL) > 0);
    }
}

#[test]
fn partial_discard_chance_draws_once_per_candidate_cell() {
    let mut world = buried_world();
    let mut recorder = Recorder::new(77);
    let feature = ore(17, 0.5);
    feature.place(&mut world, &mut recorder, IVec3::ZERO);
    let steps = 3 + 17;
    let cells = recorder.log.len() - steps;
    assert!(recorder.log[steps..].iter().all(|draw| *draw == Draws::Float));
    assert_eq!(recorder.log[..3], [Draws::Float, Draws::Below(3), Draws::Below(3)]);
    assert_eq!(world.count(ORE_CELL), cells, "a buried vein has no air exposed cell, so every drawn cell is placed");
}

#[test]
fn full_discard_chance_skips_exposed_cells_without_drawing() {
    let mut world = Grid::block_of_stone(IVec3::new(48, 12, 48), IVec3::new(-8, -12, -8));
    let mut recorder = Recorder::new(3);
    ore(33, 1.0).place(&mut world, &mut recorder, IVec3::new(0, -3, 0));
    assert_eq!(recorder.log.len(), 3 + 33);
    let placed: Vec<IVec3> = world.cells.iter().filter(|(_, block)| **block == ORE_CELL).map(|(pos, _)| *pos).collect();
    assert!(!placed.is_empty());
    for pos in placed {
        for offset in [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z] {
            assert_ne!(world.block(pos + offset), AIR, "ore at {pos} touches air");
        }
    }
}

#[test]
fn partial_discard_chance_never_places_exposed_cells_below_the_draw() {
    let mut open = Grid::block_of_stone(IVec3::new(48, 12, 48), IVec3::new(-8, -12, -8));
    let mut buried = buried_world();
    let feature = ore(33, 0.5);
    feature.place(&mut open, &mut Recorder::new(11), IVec3::new(0, -3, 0));
    feature.place(&mut buried, &mut Recorder::new(11), IVec3::new(0, -3, 0));
    assert!(open.count(ORE_CELL) <= buried.count(ORE_CELL));
}

#[test]
fn ore_without_replace_rules_places_nothing_after_three_draws() {
    let mut world = buried_world();
    let mut recorder = Recorder::new(1);
    let feature = Ore {
        count: 17,
        discard_chance: 0.0,
        rules: Vec::new(),
    };
    assert!(!feature.place(&mut world, &mut recorder, IVec3::ZERO));
    assert_eq!(recorder.log.len(), 3);
}

#[test]
fn ore_does_not_replace_other_blocks() {
    let mut world = buried_world();
    let foreign: BlockId = 9;
    let probe = IVec3::new(6, 0, 6);
    world.place(probe, foreign);
    for seed in 0..40 {
        ore(64, 0.0).place(&mut world, &mut Recorder::new(seed), IVec3::new(0, 0, 0));
    }
    assert_eq!(world.block(probe), foreign);
}

pub(super) static REGISTRY: LazyLock<BlockRegistry> = LazyLock::new(|| {
    let mut registry = BlockRegistry::new();
    registry.register_all(chorus_block::r#impl::DEFINITIONS.iter().copied());
    registry
});

fn ore_of(feature: &Feature) -> Option<&Ore> {
    match feature {
        Feature::Placed(placed) => placed.as_any().downcast_ref::<Ore>(),
        Feature::Unsupported(_) => None,
    }
}

fn loaded() -> (Loaded, BlockTable<'static>) {
    let mut table = BlockTable::new(&REGISTRY);
    let loaded = load(&mut table);
    (loaded, table)
}

#[test]
fn data_defined_piles_load_from_the_chorus_features() {
    const PILES: [&str; 5] = [
        "minecraft:hay_pile_feature",
        "minecraft:ice_pile_feature",
        "minecraft:pumpkin_pile_feature",
        "minecraft:melon_pile_feature",
        "minecraft:snow_pile_feature",
    ];
    let mut table = BlockTable::new(&REGISTRY);
    let loaded = load_with(&mut table, &PILES.map(Arc::<str>::from));
    for id in PILES {
        assert!(loaded.features.contains_key(id), "{id}");
    }
}

#[test]
fn data_defined_plants_load_from_the_chorus_features() {
    const PLANTS: [&str; 15] = [
        "minecraft:seagrass_feature",
        "minecraft:shortest_seagrass_feature",
        "minecraft:shorter_seagrass_feature",
        "minecraft:taller_seagrass_feature",
        "minecraft:kelp_feature",
        "minecraft:dead_bush_feature",
        "minecraft:melon_feature",
        "minecraft:pumpkin_feature",
        "minecraft:sweet_berry_bush_feature",
        "minecraft:reeds_feature",
        "minecraft:brown_mushroom_feature",
        "minecraft:red_mushroom_feature",
        "minecraft:grass_disc_feature",
        "minecraft:clay_feature",
        "minecraft:hay_pile_feature",
    ];
    let mut table = BlockTable::new(&REGISTRY);
    let loaded = load_with(&mut table, &PLANTS.map(Arc::<str>::from));
    for id in PLANTS {
        assert!(loaded.features.contains_key(id), "{id}");
    }
}

#[test]
fn loader_parses_every_rule_and_feature() {
    let (loaded, mut table) = loaded();
    assert!(loaded.report.unsupported_filters.is_empty(), "{:?}", loaded.report.unsupported_filters);
    let explicit = load_with_registry(&mut table, &[], &FeatureRegistry::vanilla());
    assert_eq!(explicit.report.feature_count, loaded.report.feature_count);
    assert_eq!(explicit.features.len(), loaded.features.len());
    assert_eq!(explicit.report.rule_count, loaded.report.rule_count);
    if crate::shared::structure::StructureData::shared().is_some() {
        assert!(loaded.features.contains_key("minecraft:fossil_feature"));
        assert!(loaded.features.contains_key("minecraft:deepslate_diamond_fossil_feature"));
    }
}

fn pass_order(loaded: &Loaded, pass: &str) -> Vec<&'static str> {
    let pass = rules::PASSES.iter().position(|name| *name == pass).expect("pass");
    loaded.rules.values().filter(|rule| rule.pass == pass).map(|rule| rule.identifier).collect()
}

#[test]
fn rules_are_sorted_by_identifier_within_each_pass() {
    let (loaded, _) = loaded();
    let passes: Vec<usize> = loaded.rules.values().map(|rule| rule.pass).collect();
    assert!(passes.is_sorted());
    for pass in rules::PASSES {
        let identifiers = pass_order(&loaded, pass);
        assert!(identifiers.windows(2).all(|pair| pair[0] <= pair[1]), "{} rules are not identifier-sorted", pass);
    }
}

#[test]
fn every_ore_block_exists_in_the_registry() {
    let (loaded, table) = loaded();
    let blocks = table.finish();
    let air_runtime = blocks.entry(AIR).runtime;
    for rule in loaded.rules.values() {
        if let Some(ore) = ore_of(&rule.feature) {
            for block in ore.place_blocks() {
                assert_ne!(blocks.entry(block).runtime, air_runtime, "{} places a block the registry does not have", rule.identifier);
            }
        }
    }
}

#[test]
fn legacy_stone_types_resolve_to_their_modern_blocks() {
    let (loaded, table) = loaded();
    let blocks = table.finish();
    let placed = |name: &str| {
        let rule = loaded.rules.values().find(|rule| rule.identifier == name).expect("rule");
        let ore = ore_of(&rule.feature).expect("not an ore");
        blocks.name_of(ore.place_blocks().next().expect("block"))
    };
    assert_eq!(placed("minecraft:overworld_underground_andesite_lower_feature"), "minecraft:andesite");
    assert_eq!(placed("minecraft:overworld_underground_diorite_lower_feature"), "minecraft:diorite");
    assert_eq!(placed("minecraft:overworld_underground_granite_lower_feature"), "minecraft:granite");
    assert_eq!(placed("minecraft:overworld_underground_tuff_feature"), "minecraft:tuff");
}

#[test]
fn biome_filters_resolve_through_the_biome_tags() {
    let (loaded, _) = loaded();
    let mask = |name: &str| loaded.rules.values().find(|rule| rule.identifier == name).expect("rule").biomes.clone();
    let generic_copper = mask("minecraft:overworld_underground_copper_ore_feature");
    assert!(generic_copper.contains(&Biome::PLAINS));
    assert!(!generic_copper.contains(&Biome::DRIPSTONE_CAVES));
    let dripstone_copper = mask("minecraft:dripstone_caves_underground_copper_ore_feature");
    assert!(dripstone_copper.contains(&Biome::DRIPSTONE_CAVES));
    assert_eq!(dripstone_copper.count(), 1);
    assert!(mask("minecraft:overworld_underground_emerald_ore_feature").contains(&Biome::WINDSWEPT_HILLS));
    assert!(!mask("minecraft:overworld_underground_emerald_ore_feature").contains(&Biome::PLAINS));
    assert!(mask("minecraft:overworld_underground_coal_ore_lower_feature").count() > 40);
}

#[test]
fn scatter_replays_the_documented_draws() {
    let (loaded, _) = loaded();
    let scatter = |name: &str| loaded.rules.values().find(|rule| rule.identifier == name).and_then(|rule| rule.scatter.as_ref()).expect("scatter");
    let variables = Variables {
        origin: [32.0, 0.0, -48.0],
        world: [0.0; 3],
    };

    let mut found = Vec::new();
    let origin = IVec3::new(32, 0, -48);
    let mut positions = MersenneTwister::new(555);
    let mut chunk = MersenneTwister::new(1);
    let mut draws = ScatterDraws {
        positions: &mut positions,
        molang: Some(&mut chunk),
    };
    scatter("minecraft:overworld_underground_coal_ore_lower_feature").run(&mut draws, variables, origin, &mut NoHost, |_, _, pos| found.push(pos));
    let mut words = MersenneTwister::new(555);
    let mut expected = Vec::new();
    for _ in 0..20 {
        let z = -48 + words.next_int_bounded(16);
        let y = (words.next_int_bounded(97)) + words.next_int_bounded(97);
        let x = 32 + words.next_int_bounded(16);
        expected.push(IVec3::new(x, y, z));
    }
    assert_eq!(found, expected);

    let chance = scatter("minecraft:overworld_underground_andesite_upper_feature");
    for seed in 0..30 {
        let mut found = Vec::new();
        let mut positions = MersenneTwister::new(seed);
        let mut draws = ScatterDraws {
            positions: &mut positions,
            molang: None,
        };
        chance.run(&mut draws, variables, origin, &mut NoHost, |_, _, pos| found.push(pos));
        let mut words = MersenneTwister::new(seed);
        if words.next_int_bounded(6) >= 1 {
            assert!(found.is_empty(), "seed {seed}");
        } else {
            let z = -48 + words.next_int_bounded(16);
            let y = 64 + words.next_int_bounded(64);
            let x = 32 + words.next_int_bounded(16);
            assert_eq!(found, [IVec3::new(x, y, z)], "seed {seed}");
        }
    }
}

fn run_scatter(json: serde_json::Value, positions_seed: u32, chunk_seed: Option<u32>) -> (Vec<IVec3>, MersenneTwister, MersenneTwister) {
    let scatter = Scatter::parse(&json);
    let mut positions = MersenneTwister::new(positions_seed);
    let mut chunk = MersenneTwister::new(chunk_seed.unwrap_or(0));
    let mut found = Vec::new();
    {
        let mut draws = ScatterDraws {
            positions: &mut positions,
            molang: chunk_seed.map(|_| &mut chunk),
        };
        let variables = Variables { origin: [0.0; 3], world: [0.0; 3] };
        scatter.run(&mut draws, variables, IVec3::new(16, 0, 32), &mut NoHost, |_, _, pos| found.push(pos));
    }
    (found, positions, chunk)
}

#[test]
fn math_random_in_a_rule_draws_from_the_chunk_generator_not_the_position_generator() {
    let json = serde_json::json!({
        "iterations": "math.random(2, 3)",
        "x": { "distribution": "uniform", "extent": [0, 16] },
        "y": 0,
        "z": { "distribution": "uniform", "extent": [0, 16] }
    });
    let (found, mut positions, mut chunk) = run_scatter(json.clone(), 5, Some(9));
    let count = (2.0 + MersenneTwister::new(9).next_float()).round() as usize;
    assert_eq!(found.len(), count);
    let mut expected_chunk = MersenneTwister::new(9);
    expected_chunk.next_float();
    assert_eq!(chunk.next_u32(), expected_chunk.next_u32());
    let mut expected_positions = MersenneTwister::new(5);
    expected_positions.skip(count * 2);
    assert_eq!(positions.next_u32(), expected_positions.next_u32());
    let (nested, _, _) = run_scatter(json, 5, None);
    assert_eq!(nested.len(), (2.0 + MersenneTwister::new(5).next_float()).round() as usize);
}

#[test]
fn scatter_defaults_follow_the_load_time_rewrites() {
    let none = serde_json::json!({ "x": 0, "y": 0, "z": 0 });
    assert!(run_scatter(none, 1, None).0.is_empty());
    let negative = serde_json::json!({ "iterations": -3, "x": 0, "y": 0, "z": 0 });
    assert_eq!(run_scatter(negative, 1, None).0.len(), 1);
    let zero_percent = serde_json::json!({ "iterations": 2, "scatter_chance": 0, "x": 0, "y": 0, "z": 0 });
    let (found, positions, _) = run_scatter(zero_percent, 1, None);
    assert_eq!(found.len(), 2);
    assert_eq!(positions.clone().next_u32(), MersenneTwister::new(1).next_u32());
    let reversed = serde_json::json!({ "iterations": 1, "scatter_chance": { "numerator": 3, "denominator": 2 }, "x": 0, "y": 0, "z": 0 });
    let (found, positions, _) = run_scatter(reversed, 1, None);
    assert_eq!(found.len(), 1);
    let mut expected = MersenneTwister::new(1);
    expected.next_int_bounded(1);
    assert_eq!(positions.clone().next_u32(), expected.next_u32());
}

#[test]
fn scatter_axes_default_to_x_then_z_then_y_and_round_extents() {
    let json = serde_json::json!({
        "iterations": 1,
        "x": { "distribution": "uniform", "extent": [0, 16] },
        "y": { "distribution": "uniform", "extent": [0, 10.6] },
        "z": { "distribution": "uniform", "extent": [0, 16] }
    });
    let (found, _, _) = run_scatter(json, 3, None);
    let mut words = MersenneTwister::new(3);
    let x = words.next_int_bounded(16);
    let z = words.next_int_bounded(16);
    let y = words.next_int_bounded(11);
    assert_eq!(found, [IVec3::new(16 + x, y, 32 + z)]);
}

#[test]
fn every_structure_feature_element_loads() {
    let names = crate::shared::structure::StructureData::shared().expect("structure data").feature_element_names();
    let ids: Vec<Arc<str>> = names.iter().map(|name| Arc::from(format!("minecraft:{name}"))).collect();
    let mut table = BlockTable::new(&REGISTRY);
    let loaded = load_with(&mut table, &ids);
    let missing: Vec<&Arc<str>> = ids.iter().filter(|id| !loaded.features.contains_key(id.as_ref())).collect();
    assert!(missing.iter().all(|id| id.contains("poplar")), "{missing:?}");
}

#[test]
fn chorus_feature_files_are_named_after_their_identifiers() {
    let files: Vec<_> = crate::shared::resources::CHORUS_FILES.iter().filter(|(name, _)| name.starts_with("features/")).collect();
    assert!(!files.is_empty());
    for (name, _) in &files {
        let id = format!("minecraft:{}", name.strip_prefix("features/").and_then(|file| file.strip_suffix(".json")).expect("a json feature file"));
        assert!(rules::CHORUS_FEATURES.get(&id).is_some(), "{name} does not define {id}");
    }
    assert_eq!(rules::CHORUS_FEATURES.as_object().expect("a document map").len(), files.len());
}

#[test]
fn chorus_features_keep_their_twenty_one_documents() {
    assert_eq!(rules::CHORUS_FEATURES.as_object().expect("a document map").len(), 21);
}
