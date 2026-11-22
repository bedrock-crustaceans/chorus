use super::aquatic::{Kelp, Seagrass};
use super::coral::{self, Coral};
use super::disk::Disk;
use super::geode::Geode;
use super::lake::{self, LavaLake, MonsterRoom};
use super::land::{RandomWalkPlant, SpreadPlant, WaterEdgeColumn};
use super::ore::Draw;
use super::placed::PlacedFeature;
use super::plants::{Bamboo, Cactus, Flower, FlowerMode, bamboo_bound};
use super::pool::SulfurPool;
use super::registry::FeatureRegistry;
use super::rules::{self, Feature};
use super::single_block::SingleBlock;
use super::springs::Springs;
use super::structure_template::StructureTemplate;
use super::tests::{REGISTRY, load, load_with};
use super::well::DesertWell;
use super::world::read;
use crate::shared::biome::Biome;
use crate::shared::blocks::{BlockId, BlockTable, Blocks};
use crate::shared::chunk::biome_volume::{BiomeVolume, SAMPLE_SPACING, sample_count, sample_levels, sample_origin};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_id::*;
use chorus_block::state::common::{CORAL_DIRECTION, CORAL_FAN_DIRECTION};
use glam::IVec3;
use std::collections::HashSet;
use std::sync::Arc;

const MIN_Y: i32 = -64;
const HEIGHT: i32 = 384;

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

    fn raw_words(&self) -> usize {
        self.log
            .iter()
            .map(|draw| match draw {
                Draws::Double => 1,
                _ => 1,
            })
            .sum()
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

fn table() -> BlockTable<'static> {
    BlockTable::new(&REGISTRY)
}

fn volume_world(blocks: &Blocks, water: BlockId, lava: BlockId, biome_at: impl Fn(i32) -> Biome) -> Region<'_> {
    let levels = sample_levels(HEIGHT);
    let samples: Vec<i32> = (0..sample_count(HEIGHT))
        .map(|index| biome_at(sample_origin(0, 0, MIN_Y).1 + (index % levels) as i32 * SAMPLE_SPACING).bedrock_id())
        .collect();
    let chunks = std::array::from_fn(|slot| {
        let volume = BiomeVolume::resolve(&samples, 0, slot as i32 / 3 - 1, slot as i32 % 3 - 1, MIN_Y, HEIGHT);
        ProtoChunk::with_biome_volume(slot as i32 / 3 - 1, slot as i32 % 3 - 1, MIN_Y, HEIGHT, water, lava, volume)
    });
    Region::new(0, 0, blocks, chunks)
}

fn world(blocks: &Blocks, water: BlockId, lava: BlockId) -> Region<'_> {
    volume_world(blocks, water, lava, |_| Biome::PLAINS)
}

fn fill(region: &mut Region, from_y: i32, to_y: i32, block: BlockId) {
    for x in -16..32 {
        for z in -16..32 {
            for y in from_y..=to_y {
                region.set(IVec3::new(x, y, z), block);
            }
        }
    }
}

fn find_seed(bound: i32, wanted: i32) -> u32 {
    (0..).find(|&seed| MersenneTwister::new(seed).next_int_bounded(bound) == wanted).expect("seed")
}

#[test]
fn lava_lake_draws_blobs_then_one_barrier_roll_per_upper_border_cell() {
    let mut table = table();
    let lake = LavaLake::new(&mut table);
    let (stone, lava, air) = (table.name(STONE), table.name(LAVA), table.name(AIR));
    let water = table.name(WATER);
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, MIN_Y, 62, stone);
    let mut recorder = Recorder::new(find_seed(4, 3));
    assert!(lake.place(&mut region, &mut recorder, IVec3::new(16, 67, 16)));
    assert_eq!(recorder.log[0], Draws::Below(4));
    assert!(recorder.log[1..43].iter().all(|draw| *draw == Draws::Float));
    let rolls = &recorder.log[43..];
    assert!(!rolls.is_empty() && rolls.len() < 16 * 16 * 4);
    assert!(rolls.iter().all(|draw| *draw == Draws::Below(2)));
    let mut fluid_levels = Vec::new();
    for y in 56..70 {
        let found = (8..24).any(|x| (8..24).any(|z| read(&region, IVec3::new(x, y, z)) == lava));
        if found {
            fluid_levels.push(y);
        }
    }
    assert!(!fluid_levels.is_empty() && fluid_levels.iter().all(|y| (59..=61).contains(y)), "{fluid_levels:?}");
    assert_eq!(read(&region, IVec3::new(16, 62, 16)), air);
}

#[test]
fn sulfur_pool_fills_water_below_the_mid_and_refuses_a_spike_border() {
    let mut table = table();
    let pool = SulfurPool::new(&mut table);
    let (stone, water, lava, air, sulfur, spike) = (table.name(STONE), table.name(WATER), table.name(LAVA), table.name(AIR), table.name(SULFUR), table.name(SULFUR_SPIKE));
    let blocks = table.finish();
    let origin = IVec3::new(16, 20, 16);
    let mut region = world(&blocks, water, lava);
    fill(&mut region, MIN_Y, 40, stone);
    assert!(pool.place(&mut region, &mut Recorder::new(9), origin));
    let (mut water_cells, mut air_cells, mut border) = (0, 0, None);
    for x in 8..24 {
        for z in 8..24 {
            for y in 12..28 {
                let pos = IVec3::new(x, y, z);
                let found = read(&region, pos);
                if found == water {
                    assert!(y < origin.y);
                    water_cells += 1;
                } else if found == air {
                    assert!(y >= origin.y);
                    air_cells += 1;
                } else if found == sulfur {
                    border.get_or_insert(pos);
                }
            }
        }
    }
    assert!(water_cells > 0 && air_cells > 0);
    let border = border.expect("border");
    let mut region = world(&blocks, water, lava);
    fill(&mut region, MIN_Y, 40, stone);
    region.set(border, spike);
    assert!(!pool.place(&mut region, &mut Recorder::new(9), origin));
    assert!((8..24).all(|x| (8..24).all(|z| (12..28).all(|y| read(&region, IVec3::new(x, y, z)) != water))));
}

#[test]
fn lava_lake_refuses_below_the_height_margin() {
    let mut table = table();
    let lake = LavaLake::new(&mut table);
    let (water, lava) = (table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    let mut recorder = Recorder::new(1);
    assert!(!lake.place(&mut region, &mut recorder, IVec3::new(16, MIN_Y + 4, 16)));
    assert!(recorder.log.is_empty());
}

#[test]
fn lava_origin_draws_gate_x_height_z_and_accepts_sea_level_and_below() {
    let seed = (0..).find(|&seed| MersenneTwister::new(seed).next_int_bounded(8) == 0).expect("seed");
    let mut recorder = Recorder::new(seed);
    let origin = lake::draw_lava_origin(&mut recorder, 3, -2, 63);
    assert_eq!(recorder.log[0], Draws::Below(8));
    assert_eq!(recorder.log[1], Draws::Below(9));
    assert_eq!(recorder.log[2], Draws::Below(120));
    assert!(matches!(recorder.log[3], Draws::Below(8..=127)));
    assert_eq!(recorder.log[4], Draws::Below(9));
    if let Some(origin) = origin {
        assert!((51..60 + 8 + 3).contains(&origin.x) && (-29..-20).contains(&origin.z));
        assert!(origin.y >= 60);
    }
    let closed = (0..).find(|&seed| MersenneTwister::new(seed).next_int_bounded(8) != 0).expect("seed");
    let mut recorder = Recorder::new(closed);
    assert_eq!(lake::draw_lava_origin(&mut recorder, 0, 0, 63), None);
    assert_eq!(recorder.log, [Draws::Below(8)]);
}

#[test]
fn monster_rooms_run_four_deep_and_ten_shallow_tries_below_the_generation_height() {
    let mut recorder = Recorder::new(5);
    let mut origins = Vec::new();
    lake::room_origins(&mut recorder, 3, -2, MIN_Y, |_, origin| origins.push(origin));
    assert_eq!(origins.len(), 14);
    assert!(origins[..4].iter().all(|origin| (-58..0).contains(&origin.y)));
    assert!(origins[4..].iter().all(|origin| (0..320).contains(&origin.y)));
    assert!(origins.iter().all(|origin| (56..72).contains(&origin.x) && (-24..-8).contains(&origin.z)));
    assert_eq!(recorder.log[..3], [Draws::Below(16), Draws::Below(58), Draws::Below(16)]);
    assert_eq!(recorder.log[12..15], [Draws::Below(16), Draws::Below(320), Draws::Below(16)]);
}

#[test]
fn monster_room_draws_two_radii_chest_offsets_depth_first_and_a_mob_choice() {
    let mut table = table();
    let room = MonsterRoom::new(&mut table);
    let (stone, water, lava, air) = (table.name(STONE), table.name(WATER), table.name(LAVA), table.name(AIR));
    let spawner = table.name(MOB_SPAWNER);
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, 5, 20, stone);
    region.set(IVec3::new(13, 10, 16), air);
    region.set(IVec3::new(13, 11, 16), air);
    let seed = (0..)
        .find(|&seed| {
            let mut random = MersenneTwister::new(seed);
            (random.next_int_bounded(2), random.next_int_bounded(2)) == (0, 1)
        })
        .expect("seed");
    let origin = IVec3::new(16, 10, 16);
    let mut recorder = Recorder::new(seed);
    assert!(room.place(&mut region, &mut recorder, origin));
    assert_eq!(recorder.log[..2], [Draws::Below(2), Draws::Below(2)]);
    assert_eq!(recorder.log.last(), Some(&Draws::Below(4)));
    assert_eq!(read(&region, origin), spawner);
    let tries: Vec<usize> = (0..recorder.log.len() - 1)
        .filter(|&index| recorder.log[index] == Draws::Below(7) && recorder.log[index + 1] == Draws::Below(5))
        .collect();
    assert!(!tries.is_empty());
}

#[test]
fn chest_facing_turns_away_from_the_wall_it_stands_against() {
    let walls = |north: bool, south: bool, west: bool, east: bool| {
        move |ox: i32, oz: i32| match (ox, oz) {
            (0, -1) => !north,
            (0, 1) => !south,
            (-1, 0) => !west,
            (1, 0) => !east,
            _ => true,
        }
    };
    assert_eq!(lake::chest_facing(walls(true, false, false, false), 0, -2), 0);
    assert_eq!(lake::chest_facing(walls(false, true, false, false), 0, 2), 2);
    assert_eq!(lake::chest_facing(walls(false, false, true, false), -2, 0), 3);
    assert_eq!(lake::chest_facing(walls(false, false, false, true), 2, 0), 1);
}

#[test]
fn geode_draws_start_with_the_captured_prefix() {
    let mut table = table();
    let features: &'static serde_json::Value = &rules::FEATURES;
    let geode = Geode::parse(&mut table, &features["minecraft:amethyst_geode_feature"]["body"]).expect("geode");
    let (water, lava) = (table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    let mut recorder = Recorder::new(21);
    geode.place(&mut region, &mut recorder, IVec3::new(0, 20, 0)).unwrap();
    let bounds: Vec<i32> = recorder
        .log
        .iter()
        .filter_map(|draw| if let Draws::Below(bound) = draw { Some(*bound) } else { None })
        .take(160)
        .collect();
    let expected: Vec<i32> = std::iter::once(2).chain((98..=256).rev()).collect();
    assert_eq!(bounds, expected);
    let first_float = recorder.log.iter().position(|draw| *draw == Draws::Float).expect("float");
    let sampler = Recorder {
        random: MersenneTwister::new(0),
        log: recorder.log[1..first_float].to_vec(),
    };
    assert_eq!(sampler.raw_words(), 2608);
}

#[test]
fn geode_noise_banks_spend_2608_raw_draws() {
    let mut recorder = Recorder::new(4);
    super::geode::GeodeNoise::new(&mut recorder);
    assert_eq!(recorder.raw_words(), 2608);
}

#[test]
fn geode_aborts_when_candidates_sit_in_air() {
    let mut table = table();
    let features: &'static serde_json::Value = &rules::FEATURES;
    let geode = Geode::parse(&mut table, &features["minecraft:amethyst_geode_feature"]["body"]).expect("geode");
    let (water, lava) = (table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    assert!(!geode.place(&mut region, &mut Recorder::new(3), IVec3::new(0, 20, 0)).unwrap());
}

#[test]
fn geode_fills_solid_rock_with_layers() {
    let mut table = table();
    let features: &'static serde_json::Value = &rules::FEATURES;
    let geode = Geode::parse(&mut table, &features["minecraft:amethyst_geode_feature"]["body"]).expect("geode");
    let (stone, water, lava) = (table.name(STONE), table.name(WATER), table.name(LAVA));
    let (amethyst, calcite) = (table.name(AMETHYST_BLOCK), table.name(CALCITE));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, -30, 60, stone);
    assert!(geode.place(&mut region, &mut MersenneTwister::new(8), IVec3::new(0, 20, 0)).unwrap());
    let count = |block| {
        (-16..32)
            .flat_map(|x| (-30..60).flat_map(move |y| (-16..32).map(move |z| IVec3::new(x, y, z))))
            .filter(|&pos| read(&region, pos) == block)
            .count()
    };
    assert!(count(amethyst) > 0 && count(calcite) > 0);
}

#[test]
fn cactus_spends_six_words_per_attempt() {
    let mut table = table();
    let cactus = Cactus::new(&mut table);
    let (stone, water, lava) = (table.name(STONE), table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, MIN_Y, 100, stone);
    let mut recorder = Recorder::new(2);
    assert!(cactus.place(&mut region, &mut recorder, IVec3::new(16, 50, 16)));
    assert_eq!(recorder.log, vec![Draws::Word; 60]);
}

#[test]
fn cactus_grows_on_sand_only_where_it_can_survive() {
    let mut table = table();
    let cactus = Cactus::new(&mut table);
    let (sand, shaft, water, lava) = (table.name(SAND), table.name(CACTUS), table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, MIN_Y, 62, sand);
    cactus.place(&mut region, &mut Recorder::new(6), IVec3::new(16, 63, 16));
    let mut stalks = 0;
    for x in 8..24 {
        for z in 8..24 {
            for y in 63..70 {
                if read(&region, IVec3::new(x, y, z)) == shaft {
                    stalks += 1;
                    assert!(blocks.entry(read(&region, IVec3::new(x, y - 1, z))).name.as_ref() == SAND || blocks.entry(read(&region, IVec3::new(x, y - 1, z))).name.as_ref() == CACTUS);
                }
            }
        }
    }
    assert!(stalks > 0);
}

#[test]
fn bamboo_bound_follows_the_position_hash() {
    let manual = |x: i32, z: i32| {
        let h = (z as u32).wrapping_mul(0x6ebfff5) ^ (x as u32).wrapping_mul(0x2fc20f);
        let t = (h.wrapping_mul(0x285b825).wrapping_add(0xb).wrapping_mul(h)) >> 24;
        (((t % 5) | 8) & 0xff) as i32
    };
    for (x, z) in [(0, 0), (17, -5), (-300, 1200), (5, 5)] {
        assert_eq!(bamboo_bound(x, z), manual(x, z));
        assert!((8..=12).contains(&bamboo_bound(x, z)));
    }
}

#[test]
fn bamboo_needs_its_substrate_and_stacks_leaves() {
    let mut table = table();
    let bamboo = Bamboo::new(&mut table);
    let (grass, stone, water, lava) = (table.name(GRASS_BLOCK), table.name(STONE), table.name(WATER), table.name(LAVA));
    let stalk = table.name(BAMBOO);
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, MIN_Y, 62, stone);
    assert!(!bamboo.place(&mut region, &mut Recorder::new(1), IVec3::new(16, 63, 16)));
    region.set(IVec3::new(16, 62, 16), grass);
    let mut recorder = Recorder::new(1);
    assert!(bamboo.place(&mut region, &mut recorder, IVec3::new(16, 63, 16)));
    assert_eq!(recorder.log, [Draws::Below(bamboo_bound(16, 16))]);
    assert_eq!(read(&region, IVec3::new(16, 63, 16)), stalk);
    let top = (63..90).rev().find(|&y| read(&region, IVec3::new(16, y, 16)) != 0).expect("top");
    assert!(top >= 67);
}

#[test]
fn flowers_pick_by_mode() {
    let mut table = table();
    let modes = [FlowerMode::Forest, FlowerMode::Default, FlowerMode::Swamp, FlowerMode::Plains, FlowerMode::FlowerForest];
    let flowers: Vec<Flower> = modes.iter().map(|&mode| Flower::new(&mut table, mode)).collect();
    let (dandelion, poppy, orchid, lily) = (table.name(DANDELION), table.name(POPPY), table.name(BLUE_ORCHID), table.name(LILY_OF_THE_VALLEY));
    let mut recorder = Recorder::new(12);
    let forest = flowers[0].pick(&mut recorder, IVec3::ZERO);
    assert_eq!(recorder.log, [Draws::Boolean]);
    assert!(forest == lily || forest == dandelion);
    let mut recorder = Recorder::new(12);
    let default = flowers[1].pick(&mut recorder, IVec3::ZERO);
    assert_eq!(recorder.log, [Draws::Below(3)]);
    assert!(default == dandelion || default == poppy);
    let mut recorder = Recorder::new(12);
    assert_eq!(flowers[2].pick(&mut recorder, IVec3::ZERO), orchid);
    assert!(recorder.log.is_empty());
    let mut recorder = Recorder::new(12);
    flowers[3].pick(&mut recorder, IVec3::ZERO);
    assert_eq!(recorder.log[0], Draws::Below(4));
    assert!(recorder.log.len() <= 2);
    assert!(flowers[4].pick(&mut Recorder::new(1), IVec3::ZERO) != orchid);
}

#[test]
fn flower_places_on_grass_and_refuses_solid_origin() {
    let mut table = table();
    let flower = Flower::new(&mut table, FlowerMode::Swamp);
    let (grass, orchid, stone, water, lava) = (table.name(GRASS_BLOCK), table.name(BLUE_ORCHID), table.name(STONE), table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, MIN_Y, 62, grass);
    let mut recorder = Recorder::new(3);
    assert!(flower.place(&mut region, &mut recorder, IVec3::new(16, 63, 16)));
    assert_eq!(recorder.log, vec![Draws::Word; 6]);
    assert_eq!(read(&region, IVec3::new(16, 63, 16)), orchid);
    region.set(IVec3::new(20, 63, 20), stone);
    let mut refused = Recorder::new(3);
    assert!(!flower.place(&mut region, &mut refused, IVec3::new(20, 63, 20)));
    assert!(refused.log.is_empty());
}

fn flooded_world<'a>(blocks: &'a Blocks, water: BlockId, lava: BlockId, stone: BlockId, depth: i32) -> Region<'a> {
    let mut region = world(blocks, water, lava);
    fill(&mut region, MIN_Y, 62, stone);
    fill(&mut region, 63, 62 + depth, water);
    region
}

#[test]
fn seagrass_draws_the_tall_float_then_moves_the_column() {
    let mut table = table();
    let seagrass = Seagrass::parse(&mut table, &rules::CHORUS_FEATURES["minecraft:seagrass_feature"]["body"]).expect("seagrass parses");
    let (stone, water, lava) = (table.name(STONE), table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = flooded_world(&blocks, water, lava, stone, 8);
    let mut recorder = Recorder::new(5);
    assert!(seagrass.place(&mut region, &mut recorder, IVec3::new(16, 66, 16)));
    assert_eq!(recorder.log, [vec![Draws::Float], vec![Draws::Word; 4]].concat());
}

#[test]
fn kelp_draws_its_length_then_moves_the_column() {
    let mut table = table();
    let kelp = Kelp::parse(&mut table, &rules::CHORUS_FEATURES["minecraft:kelp_feature"]["body"]);
    let (stone, water, lava) = (table.name(STONE), table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = flooded_world(&blocks, water, lava, stone, 13);
    let mut recorder = Recorder::new(5);
    assert!(kelp.place(&mut region, &mut recorder, IVec3::new(16, 70, 16)));
    assert_eq!(recorder.log, [vec![Draws::Below(10)], vec![Draws::Word; 4]].concat());
    assert!(!region.blocks.is_air(read(&region, IVec3::new(16, 63, 16))));
}

#[test]
fn land_plants_draw_two_words_per_axis_and_try() {
    let mut table = table();
    let body = |id: &str| -> &'static serde_json::Value { &rules::CHORUS_FEATURES[id]["body"] };
    let reeds = WaterEdgeColumn::parse(&mut table, body("minecraft:reeds_feature")).expect("reeds parse");
    let bush = SpreadPlant::parse(&mut table, body("minecraft:dead_bush_feature")).expect("dead bush parses");
    let melon = SpreadPlant::parse(&mut table, body("minecraft:melon_feature")).expect("melon parses");
    let pumpkins = RandomWalkPlant::parse(&mut table, body("minecraft:pumpkin_feature")).expect("pumpkins parse");
    let (stone, water, lava) = (table.name(STONE), table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, MIN_Y, 62, stone);
    let origin = IVec3::new(16, 63, 16);
    let mut recorder = Recorder::new(5);
    reeds.place(&mut region, &mut recorder, origin);
    assert_eq!(recorder.log, vec![Draws::Word; 80]);
    let mut recorder = Recorder::new(5);
    bush.place(&mut region, &mut recorder, origin);
    assert_eq!(recorder.log, vec![Draws::Word; 24]);
    let mut recorder = Recorder::new(5);
    melon.place(&mut region, &mut recorder, origin);
    assert_eq!(recorder.log, vec![Draws::Word; 384]);
    let mut recorder = Recorder::new(5);
    pumpkins.place(&mut region, &mut recorder, origin);
    assert_eq!(recorder.log, vec![Draws::Word; 576]);
}

#[test]
fn single_block_draws_the_weight_even_for_one_entry() {
    let mut table = table();
    let body: &'static serde_json::Value = &rules::FEATURES["minecraft:tall_grass_feature"]["body"];
    let single = SingleBlock::parse(&mut table, body).expect("single block");
    let (stone, water, lava) = (table.name(STONE), table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, MIN_Y, 62, stone);
    let mut recorder = Recorder::new(5);
    assert!(!single.place(&mut region, &mut recorder, IVec3::new(16, 63, 16)));
    assert_eq!(recorder.log, [Draws::Below(1)]);
}

#[test]
fn desert_well_builds_on_sand_and_draws_two_picks() {
    let mut table = table();
    let well = DesertWell::new(&mut table);
    let (sand, sandstone, water_block, lava, suspicious) = (table.name(SAND), table.name(SANDSTONE), table.name(WATER), table.name(LAVA), table.name(SUSPICIOUS_SAND));
    let stone = table.name(STONE);
    let blocks = table.finish();
    let mut region = world(&blocks, water_block, lava);
    fill(&mut region, MIN_Y, 50, stone);
    fill(&mut region, 51, 62, sand);
    let mut recorder = Recorder::new(7);
    assert!(well.place(&mut region, &mut recorder, IVec3::new(16, 70, 16)));
    assert_eq!(recorder.log, [Draws::Below(5), Draws::Below(5)]);
    assert_eq!(read(&region, IVec3::new(16, 62, 16)), water_block);
    assert_eq!(read(&region, IVec3::new(17, 62, 17)), sandstone);
    assert_eq!(read(&region, IVec3::new(17, 65, 17)), sandstone);
    let picks = (-1..=1)
        .flat_map(|dx| (-1..=1).map(move |dz| (dx, dz)))
        .filter(|&(dx, dz)| (-1..=0).any(|dy| read(&region, IVec3::new(16 + dx, 61 + dy, 16 + dz)) == suspicious))
        .count();
    assert!(picks >= 1);
}

#[test]
fn desert_well_refuses_other_ground() {
    let mut table = table();
    let well = DesertWell::new(&mut table);
    let (stone, water, lava) = (table.name(STONE), table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, MIN_Y, 62, stone);
    let mut recorder = Recorder::new(7);
    assert!(!well.place(&mut region, &mut recorder, IVec3::new(16, 63, 16)));
    assert!(recorder.log.is_empty());
}

#[test]
fn clay_discs_start_from_water_with_one_radius_draw() {
    let mut table = table();
    let clay = Disk::parse(&mut table, &rules::CHORUS_FEATURES["minecraft:clay_feature"]["body"]).expect("the clay feature parses");
    let (water, lava, dirt, clay_block) = (table.name(WATER), table.name(LAVA), table.name(DIRT), table.name(CLAY));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, 50, 59, dirt);
    region.set(IVec3::new(16, 60, 16), water);
    let origin = IVec3::new(16, 59, 16);
    region.set(origin, water);
    let mut recorder = Recorder::new(2);
    assert!(clay.place(&mut region, &mut recorder, origin));
    assert_eq!(recorder.log, [Draws::Below(2)]);
    assert_eq!(read(&region, IVec3::new(17, 59, 16)), clay_block);
    assert!(!clay.place(&mut region, &mut Recorder::new(2), IVec3::new(0, 70, 0)));
}

#[test]
fn ice_plains_spikes_rules_load() {
    let mut table = table();
    let loaded = load(&mut table);
    let runnable: Vec<&str> = loaded
        .rules
        .values()
        .filter(|rule| rule.identifier.contains("ice_plains_spikes") && rule.runs() && matches!(rule.feature, Feature::Placed(_)))
        .map(|rule| rule.identifier)
        .collect();
    assert!(runnable.contains(&"minecraft:ice_plains_spikes_first_ice_patch_feature"));
    assert!(runnable.contains(&"minecraft:ice_plains_spikes_first_ice_spike_feature"));
}

#[test]
fn rules_that_run_are_supported_end_to_end() {
    let mut table = table();
    let loaded = load(&mut table);
    let runnable: Vec<&str> = loaded
        .rules
        .values()
        .filter(|rule| rule.runs() && matches!(rule.feature, Feature::Placed(_)))
        .map(|rule| rule.identifier)
        .collect();
    for name in [
        "minecraft:desert_after_surface_desert_well_feature",
        "minecraft:overworld_amethyst_geode_feature",
        "minecraft:desert_after_surface_cactus_feature_rules",
        "minecraft:jungle_after_surface_bamboo_feature",
        "minecraft:swamp_after_surface_flowers_feature_rules",
        "minecraft:sulfur_caves_surface_sulfur_pool_with_potent_sulfur_feature_rules",
    ] {
        assert!(runnable.contains(&name), "{name}");
    }
    assert!(runnable.contains(&"minecraft:plains_after_surface_flowers_feature_rules"));
    assert!(runnable.contains(&"minecraft:flower_forest_after_surface_flowers_feature_rules"));
}

fn layered_world(blocks: &Blocks, water: BlockId, lava: BlockId, lower: Biome, upper: Biome) -> Region<'_> {
    volume_world(blocks, water, lava, |y| if y < 0 { lower.clone() } else { upper.clone() })
}

#[test]
fn biomes_are_listed_bottom_up_in_first_seen_order() {
    let mut table = table();
    let (water, lava) = (table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let region = layered_world(&blocks, water, lava, Biome::DESERT, Biome::PLAINS);
    assert_eq!(super::biome_order(&region, 0, 0), [Biome::DESERT.bedrock_id(), Biome::PLAINS.bedrock_id()]);
    let region = layered_world(&blocks, water, lava, Biome::PLAINS, Biome::DESERT);
    assert_eq!(super::biome_order(&region, 0, 0), [Biome::PLAINS.bedrock_id(), Biome::DESERT.bedrock_id()]);
    let region = world(&blocks, water, lava);
    assert_eq!(super::biome_order(&region, 0, 0), [Biome::PLAINS.bedrock_id()]);
}

#[test]
fn springs_draw_water_tries_then_lava_tries_column_height_column() {
    let mut table = table();
    let springs = Springs::new(&mut table);
    let (water, lava) = (table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    let mut recorder = Recorder::new(3);
    assert!(springs.place(&mut region, &mut recorder, IVec3::new(0, 0, 0)));
    assert_eq!(recorder.log[..3], [Draws::Below(16), Draws::Below(256), Draws::Below(16)]);
    assert_eq!(recorder.log[3 * 25..3 * 25 + 2], [Draws::Below(16), Draws::Below(240)]);
    let lava_tries = &recorder.log[3 * 25..];
    assert!(lava_tries.len() == 5 * 20);
    assert!(lava_tries.chunks(5).all(|attempt| attempt[0] == Draws::Below(16) && attempt[4] == Draws::Below(16)));
}

#[test]
fn spring_sets_a_source_only_into_a_wall_with_one_open_side() {
    let mut table = table();
    let springs = Springs::new(&mut table);
    let (stone, water, lava, air) = (table.name(STONE), table.name(WATER), table.name(LAVA), table.name(AIR));
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    fill(&mut region, 0, 20, stone);
    let spot = IVec3::new(16, 10, 16);
    region.set(spot + IVec3::NEG_Z, air);
    springs.spring(&mut region, spot, water);
    assert_eq!(read(&region, spot), water);
    let mut region = world(&blocks, water, lava);
    fill(&mut region, 0, 20, stone);
    region.set(spot + IVec3::NEG_Z, air);
    region.set(spot + IVec3::X, air);
    springs.spring(&mut region, spot, water);
    assert_eq!(read(&region, spot), stone);
    let mut region = world(&blocks, water, lava);
    fill(&mut region, 0, 20, stone);
    springs.spring(&mut region, spot, lava);
    assert_eq!(read(&region, spot), stone);
}

const SPRING_MARKERS: [&str; 6] = [
    "minecraft:sulfur",
    "minecraft:tuff",
    "minecraft:cinnabar",
    "minecraft:sulfur_spike",
    "minecraft:potent_sulfur",
    "minecraft:magma",
];

fn spring_markers(region: &Region, blocks: &Blocks, at: IVec3) -> HashSet<(i32, i32, i32)> {
    let mut found = HashSet::new();
    for x in at.x - 8..=at.x + 8 {
        for z in at.z - 8..=at.z + 8 {
            for y in at.y - 9..=at.y + 6 {
                let block = read(region, IVec3::new(x, y, z));
                if SPRING_MARKERS.contains(&blocks.name_of(block)) {
                    found.insert((x, y, z));
                }
            }
        }
    }
    found
}

fn flat_spring_world(blocks: &Blocks, water: BlockId, lava: BlockId, stone: BlockId, ground: i32) -> Region<'_> {
    let mut region = world(blocks, water, lava);
    fill(&mut region, MIN_Y, ground - 1, stone);
    region
}

#[test]
fn sulfur_spring_template_places_and_honours_constraints() {
    if crate::shared::structure::StructureData::shared().is_none() {
        return;
    }
    let mut table = table();
    let loaded = load_with(&mut table, &[Arc::from("minecraft:sulfur_spring_small_1_feature")]);
    assert!(!loaded.report.unsupported_features.contains_key("minecraft:structure_template_feature"));
    assert_eq!(loaded.features.values().filter(|feature| feature.as_any().is::<StructureTemplate>()).count(), 10);
    let feature = loaded.features.get("minecraft:sulfur_spring_small_1_feature").expect("sulfur spring feature");
    assert!(feature.as_any().is::<StructureTemplate>());
    let (stone, water, lava) = (table.name(STONE), table.name(WATER), table.name(LAVA));
    let planks = table.name("minecraft:oak_planks");
    let blocks = table.finish();
    let ground = 60;
    let at = IVec3::new(16, ground, 16);

    let mut region = flat_spring_world(&blocks, water, lava, stone, ground);
    let mut random = MersenneTwister::new(find_seed(4, 0));
    assert!(feature.place(&mut region, &mut random, at).unwrap());
    let first = spring_markers(&region, &blocks, at);
    for marker in SPRING_MARKERS {
        assert!(first.iter().any(|&(x, y, z)| blocks.name_of(read(&region, IVec3::new(x, y, z))) == marker), "{marker}");
    }

    let mut turned = flat_spring_world(&blocks, water, lava, stone, ground);
    let mut random = MersenneTwister::new(find_seed(4, 1));
    assert!(feature.place(&mut turned, &mut random, at).unwrap());
    let second = spring_markers(&turned, &blocks, at);
    assert_eq!(first.len(), second.len());
    assert_ne!(first, second);
    for &(x, _, z) in &second {
        assert!((x - at.x).abs() <= 5 && (z - at.z).abs() <= 5);
    }

    let mut blocked = world(&blocks, water, lava);
    fill(&mut blocked, MIN_Y, ground - 1, planks);
    let mut random = MersenneTwister::new(find_seed(4, 0));
    assert!(!feature.place(&mut blocked, &mut random, at).unwrap());
    assert!(spring_markers(&blocked, &blocks, at).is_empty());

    let mut uneven = flat_spring_world(&blocks, water, lava, stone, ground);
    for x in at.x + 3..=at.x + 8 {
        for z in at.z - 1..=at.z + 1 {
            for y in ground..=ground + 7 {
                uneven.set(IVec3::new(x, y, z), stone);
            }
        }
    }
    let mut random = MersenneTwister::new(find_seed(4, 0));
    assert!(!feature.place(&mut uneven, &mut random, at).unwrap());
    assert!(spring_markers(&uneven, &blocks, at).is_empty());
}

#[test]
fn coral_feature_family_loads_from_the_warm_ocean_rules() {
    let mut table = table();
    let loaded = load_with(&mut table, &[]);
    for id in [
        "minecraft:coral_feature",
        "minecraft:coral_crust_feature",
        "minecraft:coral_hang_feature",
        "minecraft:sea_anemone_feature",
    ] {
        assert!(loaded.features.contains_key(id), "{id}");
    }
    for identifier in [
        "minecraft:warm_ocean_after_surface_coral_feature",
        "minecraft:warm_ocean_after_surface_coral_crust_feature",
        "minecraft:warm_ocean_after_surface_coral_hang_feature",
        "minecraft:warm_ocean_after_surface_sea_anemone_feature",
    ] {
        let rule = loaded.rules.values().find(|rule| rule.identifier == identifier).expect(identifier);
        assert!(matches!(rule.feature, Feature::Placed(_)), "{identifier}");
    }
    assert_eq!(coral::template_count(), 11);
}

#[test]
fn coral_places_blocks_and_directional_fans_on_the_water_floor() {
    let mut table = table();
    let coral = Coral::new(&mut table);
    let (stone, water, lava) = (table.name(STONE), table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let families: Vec<&str> = [TUBE_CORAL_BLOCK, BRAIN_CORAL_BLOCK, BUBBLE_CORAL_BLOCK, FIRE_CORAL_BLOCK, HORN_CORAL_BLOCK].to_vec();
    let fans: Vec<&str> = [TUBE_CORAL_FAN, BRAIN_CORAL_FAN, BUBBLE_CORAL_FAN, FIRE_CORAL_FAN, HORN_CORAL_FAN].to_vec();
    let wall_fans: Vec<&str> = [TUBE_CORAL_WALL_FAN, BRAIN_CORAL_WALL_FAN, BUBBLE_CORAL_WALL_FAN, FIRE_CORAL_WALL_FAN, HORN_CORAL_WALL_FAN].to_vec();
    let (mut saw_block, mut saw_fan, mut saw_wall_fan) = (false, false, false);
    for seed in 0..48 {
        let mut region = world(&blocks, water, lava);
        fill(&mut region, MIN_Y, 50, stone);
        fill(&mut region, 51, 62, water);
        let mut random = MersenneTwister::new(seed);
        assert!(coral.place(&mut region, &mut random, IVec3::new(16, 51, 16)));
        for x in 4..29 {
            for z in 4..29 {
                for y in 51..67 {
                    let pos = IVec3::new(x, y, z);
                    let id = read(&region, pos);
                    let name = blocks.name_of(id);
                    if families.contains(&name) {
                        saw_block = true;
                    }
                    if fans.contains(&name) {
                        saw_fan = true;
                        assert!((0..2).contains(&blocks.int(id, &CORAL_FAN_DIRECTION).expect("coral_fan_direction")));
                    }
                    if wall_fans.contains(&name) {
                        saw_wall_fan = true;
                        assert!((0..4).contains(&blocks.int(id, &CORAL_DIRECTION).expect("coral_direction")));
                    }
                }
            }
        }
    }
    assert!(saw_block && saw_fan && saw_wall_fan);
}

#[derive(Debug)]
struct FillFeature {
    block: BlockId,
}

impl PlacedFeature for FillFeature {
    fn run(&self, region: &mut Region, _random: &mut MersenneTwister, pos: IVec3) -> crate::shared::structure::Probed<bool> {
        Ok(region.set(pos, self.block))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

const FILL_DOCUMENTS: &str = r#"{
    "test:fill_stone": {
        "format_version": "1.0.0",
        "test:fill_feature": {"description": {"identifier": "test:fill_stone"}, "block": "minecraft:stone"}
    }
}"#;

#[test]
fn custom_feature_types_load_from_extra_documents() {
    let mut registry = FeatureRegistry::vanilla().with_documents(FILL_DOCUMENTS);
    registry.register("test:fill_feature", |loader, doc| {
        let name = doc.body["block"].as_str().ok_or(doc.blocked())?;
        Ok(super::rules::arc(FillFeature { block: loader.table().name(name) }))
    });
    let mut table = table();
    let stone = table.name(STONE);
    let water = table.name(WATER);
    let lava = table.name(LAVA);
    let loaded = rules::load_with_registry(&mut table, &[Arc::from("test:fill_stone")], &registry);
    let feature = loaded.features.get("test:fill_stone").expect("custom feature loads");
    assert!(feature.as_any().is::<FillFeature>());
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    let at = IVec3::new(16, 60, 16);
    assert_ne!(read(&region, at), stone);
    assert!(feature.place(&mut region, &mut MersenneTwister::new(1), at).unwrap());
    assert_eq!(read(&region, at), stone);
}

#[test]
fn unregistered_custom_feature_types_stay_blocked() {
    let registry = FeatureRegistry::vanilla().with_documents(FILL_DOCUMENTS);
    let mut table = table();
    let loaded = rules::load_with_registry(&mut table, &[Arc::from("test:fill_stone")], &registry);
    assert!(!loaded.features.contains_key("test:fill_stone"));
}
