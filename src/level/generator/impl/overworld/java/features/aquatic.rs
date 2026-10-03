use glam::IVec3;

use super::super::blocks::{BlockId, BlockTable, State, int, key, state, text};
use super::super::random::RandomSource;
use super::basic::simple;
use super::key::Key;
use super::placement::{Modifier, Placed, count, filter};
use super::predicate::{BlockPredicate, Direction, coral_direction};
use super::provider::{IntProvider, StateProvider};
use super::region::Heightmap;
use super::tree::shuffle;
use super::vegetation::BlockColumn;
use super::{Context, Feature};
use crate::block::block_id::*;
use crate::block::state::common::{CLUSTER_COUNT, CORAL_DIRECTION, KELP_AGE, SEA_GRASS_TYPE};

struct Coral {
    block: &'static str,
    plant: &'static str,
    fan: &'static str,
    wall_fan: &'static str,
}

const CORALS: [Coral; 5] = [
    Coral {
        block: TUBE_CORAL_BLOCK,
        plant: TUBE_CORAL,
        fan: TUBE_CORAL_FAN,
        wall_fan: TUBE_CORAL_WALL_FAN,
    },
    Coral {
        block: BRAIN_CORAL_BLOCK,
        plant: BRAIN_CORAL,
        fan: BRAIN_CORAL_FAN,
        wall_fan: BRAIN_CORAL_WALL_FAN,
    },
    Coral {
        block: BUBBLE_CORAL_BLOCK,
        plant: BUBBLE_CORAL,
        fan: BUBBLE_CORAL_FAN,
        wall_fan: BUBBLE_CORAL_WALL_FAN,
    },
    Coral {
        block: FIRE_CORAL_BLOCK,
        plant: FIRE_CORAL,
        fan: FIRE_CORAL_FAN,
        wall_fan: FIRE_CORAL_WALL_FAN,
    },
    Coral {
        block: HORN_CORAL_BLOCK,
        plant: HORN_CORAL,
        fan: HORN_CORAL_FAN,
        wall_fan: HORN_CORAL_WALL_FAN,
    },
];

pub fn coral_tree(ctx: &mut Context, random: &mut dyn RandomSource, feature: &Placed, origin: IVec3) -> bool {
    let mut pos = origin;
    let trunk_height = random.next_int_bounded(3) + 1;
    for _ in 0..trunk_height {
        if !feature.place(ctx, random, pos, None) {
            return true;
        }
        pos += IVec3::Y;
    }
    let top = pos;
    let branches = random.next_int_bounded(3) + 2;
    let mut directions = Direction::HORIZONTAL;
    shuffle(&mut directions, random);
    for &direction in &directions[..branches as usize] {
        let mut pos = top + direction.offset();
        let branch_height = random.next_int_bounded(5) + 2;
        let mut segment = 0;
        let mut j = 0;
        while j < branch_height && feature.place(ctx, random, pos, None) {
            segment += 1;
            pos += IVec3::Y;
            if j == 0 || (segment >= 2 && random.next_float() < 0.25) {
                pos += direction.offset();
                segment = 0;
            }
            j += 1;
        }
    }
    true
}

pub fn coral_claw(ctx: &mut Context, random: &mut dyn RandomSource, feature: &Placed, origin: IVec3) -> bool {
    if !feature.place(ctx, random, origin, None) {
        return false;
    }
    let claw = Direction::HORIZONTAL[random.next_int_bounded(4) as usize];
    let branches = random.next_int_bounded(2) + 2;
    let mut directions = [claw, claw.clockwise(), claw.counter_clockwise()];
    shuffle(&mut directions, random);
    for &branch in &directions[..branches as usize] {
        let mut pos = origin;
        let sideways = random.next_int_bounded(2) + 1;
        pos += branch.offset();
        let (segment, inway) = if branch == claw {
            (claw, random.next_int_bounded(3) + 2)
        } else {
            pos += IVec3::Y;
            let segment = [branch, Direction::Up][random.next_int_bounded(2) as usize];
            (segment, random.next_int_bounded(3) + 3)
        };
        let mut i = 0;
        while i < sideways && feature.place(ctx, random, pos, None) {
            pos += segment.offset();
            i += 1;
        }
        pos -= segment.offset();
        pos += IVec3::Y;
        for _ in 0..inway {
            pos += claw.offset();
            if !feature.place(ctx, random, pos, None) {
                break;
            }
            if random.next_float() < 0.25 {
                pos += IVec3::Y;
            }
        }
    }
    true
}

fn water(table: &mut BlockTable, offset: IVec3) -> BlockPredicate {
    BlockPredicate::MatchingBlocks(offset, vec![table.kind(WATER)])
}

fn wet(table: &mut BlockTable, name: &'static str, states: &[State]) -> BlockId {
    let dry = table.get(name, states);
    table.flooded(dry)
}

fn sea_pickle(table: &mut BlockTable) -> Feature {
    for count in 0..4 {
        wet(table, SEA_PICKLE, &[state(CLUSTER_COUNT, int(count))]);
    }
    let source = wet(table, SEA_PICKLE, &[]);
    simple(StateProvider::RandomizedInt {
        source: Box::new(source.into()),
        property: key(CLUSTER_COUNT),
        values: IntProvider::Uniform(0, 3),
    })
}

fn coral_decoration(table: &mut BlockTable) -> Feature {
    let mut corals: Vec<(BlockId, i32)> = CORALS.iter().map(|coral| (wet(table, coral.plant, &[]), 1)).collect();
    corals.extend(CORALS.iter().map(|coral| (wet(table, coral.fan, &[]), 1)));
    let top = Feature::WeightedSelector(vec![
        (Placed::new(simple(StateProvider::Weighted(corals)), Vec::new()), 20),
        (Placed::new(sea_pickle(table), Vec::new()), 3),
        (Placed::new(Feature::NoOp, Vec::new()), 57),
    ]);
    let mut features = vec![Placed::new(top, vec![Modifier::Offset(0.into(), 1.into(), 0.into())])];
    for direction in [Direction::North, Direction::East, Direction::South, Direction::West] {
        let facing = state(CORAL_DIRECTION, int(coral_direction(direction)));
        let fans = CORALS.iter().map(|coral| (wet(table, coral.wall_fan, std::slice::from_ref(&facing)), 1)).collect();
        let offset = direction.offset();
        let placement = vec![
            Modifier::RandomChance(0.2),
            Modifier::Offset(offset.x.into(), offset.y.into(), offset.z.into()),
            filter(water(table, IVec3::ZERO)),
        ];
        features.push(Placed::new(simple(StateProvider::Weighted(fans)), placement));
    }
    Feature::Overlay(features)
}

fn coral_block(table: &mut BlockTable, coral: &Coral) -> Feature {
    let block = table.name(coral.block);
    Feature::Overlay(vec![Placed::new(simple(block), Vec::new()), Placed::new(coral_decoration(table), Vec::new())])
}

fn coral_allowed(table: &mut BlockTable) -> Modifier {
    let corals = super::super::tags::Tag::Corals;
    let here = BlockPredicate::AnyOf(vec![water(table, IVec3::ZERO), BlockPredicate::MatchingTag(IVec3::ZERO, corals)]);
    filter(BlockPredicate::AllOf(vec![here, water(table, IVec3::Y)]))
}

fn warm_ocean_vegetation(table: &mut BlockTable) -> Feature {
    let mut features = Vec::new();
    for coral in &CORALS {
        let tree = Placed::new(coral_block(table, coral), vec![coral_allowed(table)]);
        features.push(Placed::new(Feature::CoralTree(Box::new(tree)), Vec::new()));
        let claw = Placed::new(coral_block(table, coral), vec![coral_allowed(table)]);
        features.push(Placed::new(Feature::CoralClaw(Box::new(claw)), Vec::new()));
        let mushroom = vec![
            Modifier::Offset(0.into(), IntProvider::Uniform(-3, -1), 0.into()),
            Modifier::Cuboid {
                xz: IntProvider::Uniform(3, 5),
                y: IntProvider::Uniform(3, 5),
                edges: false,
                interior: false,
            },
            Modifier::RandomChance(0.9),
            coral_allowed(table),
        ];
        features.push(Placed::new(coral_block(table, coral), mushroom));
    }
    Feature::SimpleRandomSelector(features)
}

fn seagrass(table: &mut BlockTable, tall_percentage: i32) -> Feature {
    table.get(SEAGRASS, &[state(SEA_GRASS_TYPE, text("double_top"))]);
    let tall = table.get(SEAGRASS, &[state(SEA_GRASS_TYPE, text("double_bot"))]);
    let tall = Placed::new(simple(tall), vec![filter(water(table, IVec3::Y))]);
    let short = Placed::new(simple(table.name(SEAGRASS)), Vec::new());
    Feature::WeightedSelector(vec![(tall, tall_percentage), (short, 100 - tall_percentage)])
}

fn kelp(table: &mut BlockTable) -> Feature {
    for age in 20..=23 {
        table.get(KELP, &[state(KELP_AGE, int(age))]);
    }
    let head = StateProvider::RandomizedInt {
        source: Box::new(table.name(KELP).into()),
        property: key(KELP_AGE),
        values: IntProvider::Uniform(20, 23),
    };
    Feature::BlockColumn(BlockColumn {
        layers: vec![(IntProvider::Uniform(0, 9), table.name(KELP).into()), (1.into(), head)],
        direction: Direction::Up,
        allowed: BlockPredicate::AllOf(vec![water(table, IVec3::ZERO), water(table, IVec3::Y)]),
        prioritize_tip: true,
    })
}

fn seagrass_placement(table: &mut BlockTable, tries: i32) -> Vec<Modifier> {
    vec![
        Modifier::InSquare,
        count(tries),
        Modifier::Offset(IntProvider::triangle(7), IntProvider::triangle(0), IntProvider::triangle(7)),
        Modifier::Heightmap(Heightmap::OceanFloor),
        filter(water(table, IVec3::ZERO)),
        Modifier::Biome,
    ]
}

pub fn define(key: Key, table: &mut BlockTable) -> Option<Placed> {
    let (feature, placement) = match key {
        Key::SeagrassWarm => (seagrass(table, 30), seagrass_placement(table, 80)),
        Key::SeagrassNormal => (seagrass(table, 30), seagrass_placement(table, 48)),
        Key::SeagrassCold => (seagrass(table, 30), seagrass_placement(table, 32)),
        Key::SeagrassRiver => (seagrass(table, 40), seagrass_placement(table, 48)),
        Key::SeagrassSwamp => (seagrass(table, 60), seagrass_placement(table, 64)),
        Key::SeagrassDeepWarm => (seagrass(table, 80), seagrass_placement(table, 80)),
        Key::SeagrassDeep => (seagrass(table, 80), seagrass_placement(table, 48)),
        Key::SeagrassDeepCold => (seagrass(table, 80), seagrass_placement(table, 40)),
        Key::SeaPickle => {
            let mut placement = vec![Modifier::Rarity(16)];
            placement.extend(seagrass_placement(table, 20));
            (sea_pickle(table), placement)
        }
        Key::KelpCold | Key::KelpWarm => {
            let cannot_support = BlockPredicate::MatchingTag(IVec3::NEG_Y, super::super::tags::Tag::CannotSupportKelp).not();
            let allowed = BlockPredicate::AllOf(vec![water(table, IVec3::ZERO), water(table, IVec3::Y), BlockPredicate::SturdyFace(IVec3::NEG_Y), cannot_support]);
            let ratio = if key == Key::KelpCold { 120 } else { 80 };
            let placement = vec![
                Modifier::NoiseBasedCount { ratio, factor: 80.0, offset: 0.0 },
                Modifier::InSquare,
                Modifier::Heightmap(Heightmap::OceanFloor),
                filter(allowed),
                Modifier::Biome,
            ];
            (kelp(table), placement)
        }
        Key::WarmOceanVegetation => {
            let placement = vec![
                Modifier::NoiseBasedCount {
                    ratio: 20,
                    factor: 400.0,
                    offset: 0.0,
                },
                Modifier::InSquare,
                Modifier::Heightmap(Heightmap::OceanFloorWg),
                Modifier::Biome,
            ];
            (warm_ocean_vegetation(table), placement)
        }
        _ => return None,
    };
    Some(Placed::new(feature, placement))
}
