use glam::IVec3;

use super::super::blocks::{BlockId, BlockTable, flag, int, key, state, text};
use super::super::tags::Tag;
use super::placement::{Placed, filter};
use super::predicate::{BlockPredicate, Direction, legacy_direction, vine_bit};
use super::provider::{IntProvider, StateProvider};
use super::tree::{Decorator, FallenTree, FeatureSize, FoliageKind, FoliagePlacer, HugeMushroom, MangroveRoots, Tree, TrunkKind, TrunkPlacer};
use super::{Feature, vegetation};
use crate::block::block_id::*;
use crate::block::state::common::{
    AGE_3, CREAKING_HEART_STATE, DIRECTION, GROWTH, HANGING, HUGE_MUSHROOM_BITS, MINECRAFT_CARDINAL_DIRECTION, NATURAL, PILLAR_AXIS, PROPAGULE_STAGE, TIP, VINE_DIRECTION_BITS,
};

pub fn log(table: &mut BlockTable, name: &'static str) -> BlockId {
    table.get(name, &[state(PILLAR_AXIS, text("x"))]);
    table.get(name, &[state(PILLAR_AXIS, text("z"))]);
    table.get(name, &[state(PILLAR_AXIS, text("y"))])
}

pub fn leaves(table: &mut BlockTable, name: &'static str) -> BlockId {
    let dry = table.name(name);
    table.flooded(dry);
    dry
}

fn vines(table: &mut BlockTable) -> [BlockId; 4] {
    [Direction::East, Direction::West, Direction::South, Direction::North].map(|face| table.get(VINE, &[state(VINE_DIRECTION_BITS, int(vine_bit(face)))]))
}

fn soil_beneath_tree(table: &mut BlockTable) -> StateProvider {
    let replaceable = BlockPredicate::MatchingTag(IVec3::ZERO, Tag::CannotReplaceBelowTreeTrunk).not();
    StateProvider::RuleBased {
        fallback: None,
        rules: vec![(replaceable, table.name(DIRT).into())],
    }
}

fn podzol_beneath_tree(table: &mut BlockTable) -> StateProvider {
    StateProvider::RuleBased {
        fallback: None,
        rules: vec![(BlockPredicate::MatchingTag(IVec3::ZERO, Tag::BeneathTreePodzolReplaceable), table.name(PODZOL).into())],
    }
}

fn trunk(base_height: i32, rand_a: i32, rand_b: i32, kind: TrunkKind) -> TrunkPlacer {
    TrunkPlacer { base_height, rand_a, rand_b, kind }
}

fn foliage(radius: impl Into<IntProvider>, offset: impl Into<IntProvider>, kind: FoliageKind) -> FoliagePlacer {
    FoliagePlacer {
        radius: radius.into(),
        offset: offset.into(),
        kind,
    }
}

fn tree(table: &mut BlockTable, log_name: &'static str, trunk_placer: TrunkPlacer, foliage_provider: StateProvider, foliage_placer: FoliagePlacer, size: FeatureSize) -> Tree {
    Tree {
        trunk: log(table, log_name).into(),
        trunk_placer,
        foliage: foliage_provider,
        foliage_placer,
        roots: None,
        size,
        decorators: Vec::new(),
        ignore_vines: false,
        below_trunk: soil_beneath_tree(table),
        vine: table.kind(VINE),
    }
}

fn straight_blob(table: &mut BlockTable, log_name: &'static str, leaves_name: &'static str, base: i32, a: i32, b: i32, radius: i32) -> Tree {
    let foliage_state = leaves(table, leaves_name).into();
    tree(
        table,
        log_name,
        trunk(base, a, b, TrunkKind::Straight),
        foliage_state,
        foliage(radius, 0, FoliageKind::Blob { height: 3 }),
        FeatureSize::two(1, 0, 1),
    )
}

fn ignoring_vines(mut tree: Tree) -> Tree {
    tree.ignore_vines = true;
    tree
}

fn jungle(table: &mut BlockTable) -> Tree {
    straight_blob(table, JUNGLE_LOG, JUNGLE_LEAVES, 4, 8, 0, 2)
}

fn fancy_oak(table: &mut BlockTable) -> Tree {
    let foliage_state = leaves(table, OAK_LEAVES).into();
    let size = FeatureSize::TwoLayers {
        limit: 0,
        lower: 0,
        upper: 0,
        min_clipped: Some(4),
    };
    ignoring_vines(tree(
        table,
        OAK_LOG,
        trunk(3, 11, 0, TrunkKind::Fancy),
        foliage_state,
        foliage(2, 4, FoliageKind::Fancy { height: 4 }),
        size,
    ))
}

fn dark_oak_like(table: &mut BlockTable, log_name: &'static str, leaves_name: &'static str) -> Tree {
    let foliage_state = leaves(table, leaves_name).into();
    let size = FeatureSize::ThreeLayers {
        limit: 1,
        upper_limit: 1,
        lower: 0,
        middle: 1,
        upper: 2,
    };
    ignoring_vines(tree(table, log_name, trunk(6, 2, 1, TrunkKind::DarkOak), foliage_state, foliage(0, 0, FoliageKind::DarkOak), size))
}

fn cherry(table: &mut BlockTable) -> Tree {
    let kind = TrunkKind::Cherry {
        branch_count: IntProvider::Weighted(vec![(1.into(), 1), (2.into(), 1), (3.into(), 1)]),
        branch_horizontal_length: IntProvider::Uniform(2, 4),
        branch_start: (-4, -3),
        branch_end: IntProvider::Uniform(-1, 0),
    };
    let foliage_state = leaves(table, CHERRY_LEAVES).into();
    let placer = foliage(
        4,
        0,
        FoliageKind::Cherry {
            height: 5.into(),
            wide_bottom_hole: 0.25,
            corner_hole: 0.5,
            hanging: 0.16666667,
            hanging_extension: 0.33333334,
        },
    );
    ignoring_vines(tree(table, CHERRY_LOG, trunk(7, 1, 0, kind), foliage_state, placer, FeatureSize::two(1, 0, 2)))
}

fn poplar(table: &mut BlockTable, leaves_name: &'static str) -> Tree {
    let kind = TrunkKind::Poplar {
        trunk_above_branches: 4.into(),
        branch_amount: IntProvider::Uniform(1, 4),
    };
    let radius = IntProvider::Weighted(vec![(5.into(), 5), (6.into(), 5), (7.into(), 1), (8.into(), 1)]);
    let foliage_state = leaves(table, leaves_name).into();
    let placer = foliage(
        radius,
        0,
        FoliageKind::Poplar {
            height: IntProvider::Uniform(5, 6),
            side_hole: 0.15,
        },
    );
    let mut tree = ignoring_vines(tree(table, POPLAR_LOG, trunk(7, 4, 0, kind), foliage_state, placer, FeatureSize::two(1, 0, 2)));
    tree.decorators.push(shelf_mushroom(table, 0.4));
    tree
}

fn with_decorators(mut tree: Tree, decorators: Vec<Decorator>) -> Tree {
    tree.decorators.splice(0..0, decorators);
    tree
}

fn beehive(table: &mut BlockTable, probability: f32) -> Decorator {
    Decorator::Beehive {
        probability,
        nest: table.get(BEE_NEST, &[state(DIRECTION, int(legacy_direction(Direction::South)))]),
    }
}

fn leaf_litter_provider(table: &mut BlockTable, min: i32, max: i32) -> StateProvider {
    StateProvider::Weighted(vegetation::segmented(table, LEAF_LITTER, min, max))
}

fn leaf_litter(table: &mut BlockTable) -> Vec<Decorator> {
    vec![
        Decorator::PlaceOnGround {
            tries: 96,
            radius: 4,
            height: 2,
            provider: leaf_litter_provider(table, 1, 3),
        },
        Decorator::PlaceOnGround {
            tries: 150,
            radius: 2,
            height: 2,
            provider: leaf_litter_provider(table, 1, 4),
        },
    ]
}

fn shelf_mushroom(table: &mut BlockTable, probability: f32) -> Decorator {
    let mut states = Vec::new();
    for age in 0..=1 {
        for direction in Direction::HORIZONTAL {
            states.push(table.get(SHELF_MUSHROOM, &[state(GROWTH, int(age)), state(MINECRAFT_CARDINAL_DIRECTION, text(direction.name()))]));
        }
    }
    Decorator::ShelfMushroom { probability, states }
}

fn leave_vine(table: &mut BlockTable, probability: f32) -> Decorator {
    Decorator::LeaveVine { probability, vines: vines(table) }
}

fn trunk_vine(table: &mut BlockTable) -> Decorator {
    Decorator::TrunkVine { vines: vines(table) }
}

fn cocoa(table: &mut BlockTable, probability: f32) -> Decorator {
    let mut states = Vec::new();
    for age in 0..=2 {
        for direction in Direction::HORIZONTAL {
            states.push(table.get(COCOA, &[state(AGE_3, int(age)), state(DIRECTION, int(legacy_direction(direction)))]));
        }
    }
    Decorator::Cocoa { probability, states }
}

fn pale_moss(table: &mut BlockTable) -> Decorator {
    Decorator::PaleMoss {
        leaves: 0.15,
        trunk: 0.4,
        ground: 0.8,
        patch: Box::new(vegetation::pale_moss_patch(table)),
        hanging: table.get(PALE_HANGING_MOSS, &[state(TIP, flag(false))]),
        tip: table.get(PALE_HANGING_MOSS, &[state(TIP, flag(true))]),
    }
}

fn alter_ground_podzol(table: &mut BlockTable) -> Decorator {
    Decorator::AlterGround(podzol_beneath_tree(table))
}

fn mangrove(table: &mut BlockTable, base: i32, rand_b: i32, steps: i32, root_offset: (i32, i32), size_limit: i32) -> Tree {
    let kind = TrunkKind::UpwardsBranching {
        extra_branch_steps: IntProvider::Uniform(1, steps),
        branch_chance: 0.5,
        extra_branch_length: IntProvider::Uniform(0, 1),
        can_grow_through: Tag::MangroveLogsCanGrowThrough,
    };
    let foliage_state = leaves(table, MANGROVE_LEAVES).into();
    let placer = foliage(3, 0, FoliageKind::RandomSpread { height: 2.into(), attempts: 70 });
    let mut tree = ignoring_vines(tree(table, MANGROVE_LOG, trunk(base, 1, rand_b, kind), foliage_state, placer, FeatureSize::two(size_limit, 0, 2)));
    let roots = table.name(MANGROVE_ROOTS);
    table.flooded(roots);
    tree.roots = Some(MangroveRoots {
        trunk_offset_y: IntProvider::Uniform(root_offset.0, root_offset.1),
        root: table.name(MANGROVE_ROOTS),
        above_root: Some((table.name(MOSS_CARPET), 0.5)),
        can_grow_through: Tag::MangroveRootsCanGrowThrough,
        muddy_roots_in: vec![table.kind(MUD), table.kind(MUDDY_MANGROVE_ROOTS)],
        muddy_roots: log(table, MUDDY_MANGROVE_ROOTS),
        max_root_width: 8,
        max_root_length: 15,
        random_skew_chance: 0.2,
    });
    for age in 0..=4 {
        table.get(MANGROVE_PROPAGULE, &[state(PROPAGULE_STAGE, int(age)), state(HANGING, flag(true))]);
    }
    let propagule = StateProvider::RandomizedInt {
        source: Box::new(table.get(MANGROVE_PROPAGULE, &[state(HANGING, flag(true))]).into()),
        property: key(PROPAGULE_STAGE),
        values: IntProvider::Uniform(0, 4),
    };
    tree.decorators = vec![
        leave_vine(table, 0.125),
        Decorator::AttachedToLeaves {
            probability: 0.14,
            exclusion_xz: 1,
            exclusion_y: 0,
            provider: propagule,
            required_empty: 2,
            directions: vec![Direction::Down],
        },
        beehive(table, 0.01),
    ];
    tree
}

fn fallen_log(table: &mut BlockTable, log_name: &'static str, min: i32, max: i32, stump_vines: bool) -> FallenTree {
    let mushrooms = StateProvider::Weighted(vec![(table.name(RED_MUSHROOM), 2), (table.name(BROWN_MUSHROOM), 1)]);
    FallenTree {
        trunk: log(table, log_name).into(),
        length: IntProvider::Uniform(min, max),
        stump_decorators: if stump_vines { vec![trunk_vine(table)] } else { Vec::new() },
        log_decorators: vec![Decorator::AttachedToLogs {
            probability: 0.1,
            provider: mushrooms,
            directions: vec![Direction::Up],
        }],
    }
}

const STEM_SIDES: i32 = 10;

fn cap_bits(index: usize) -> i32 {
    let (up, west, east, north, south) = (index & 16 != 0, index & 8 != 0, index & 4 != 0, index & 2 != 0, index & 1 != 0);
    let row = if north {
        0
    } else if south {
        2
    } else {
        1
    };
    let column = if west {
        0
    } else if east {
        2
    } else {
        1
    };
    if row == 1 && column == 1 && !up { 0 } else { 1 + row * 3 + column }
}

fn huge_mushroom(table: &mut BlockTable, red: bool) -> HugeMushroom {
    let cap_name = if red { RED_MUSHROOM_BLOCK } else { BROWN_MUSHROOM_BLOCK };
    let cap = (0..32).map(|index| table.get(cap_name, &[state(HUGE_MUSHROOM_BITS, int(cap_bits(index)))])).collect();
    let stem = table.get(MUSHROOM_STEM, &[state(HUGE_MUSHROOM_BITS, int(STEM_SIDES))]);
    let tag = if red { Tag::HugeRedMushroomCanPlaceOn } else { Tag::HugeBrownMushroomCanPlaceOn };
    HugeMushroom {
        red,
        cap,
        stem,
        radius: if red { 2 } else { 3 },
        can_place_on: BlockPredicate::MatchingTag(IVec3::ZERO, tag),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species {
    Oak,
    FancyOak,
    SwampOak,
    Birch,
    SuperBirch,
    DarkOak,
    PaleOak,
    CreakingPaleOak,
    Acacia,
    Cherry,
    Spruce,
    Pine,
    MegaSpruce,
    MegaPine,
    Jungle,
    MegaJungle,
    JungleBush,
    Azalea,
    Mangrove,
    TallMangrove,
    RedPoplar,
    OrangePoplar,
    YellowPoplar,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fallen {
    Oak,
    Birch,
    SuperBirch,
    Jungle,
    Spruce,
    Poplar,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Standing {
    pub species: Species,
    pub bees: Option<f32>,
    pub leaf_litter: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TreeFeature {
    Standing(Standing),
    Fallen(Fallen),
    HugeBrownMushroom,
    HugeRedMushroom,
}

impl Species {
    pub const fn with_bees(self, chance: f32) -> Standing {
        Standing {
            species: self,
            bees: Some(chance),
            leaf_litter: false,
        }
    }

    pub const fn with_leaf_litter(self) -> Standing {
        Standing {
            species: self,
            bees: None,
            leaf_litter: true,
        }
    }

    fn sapling(self) -> &'static str {
        match self {
            Self::DarkOak => DARK_OAK_SAPLING,
            Self::PaleOak | Self::CreakingPaleOak => PALE_OAK_SAPLING,
            Self::Acacia => ACACIA_SAPLING,
            Self::Mangrove | Self::TallMangrove => MANGROVE_PROPAGULE,
            Self::Cherry => CHERRY_SAPLING,
            Self::RedPoplar | Self::OrangePoplar | Self::YellowPoplar => POPLAR_SAPLING,
            Self::Birch | Self::SuperBirch => BIRCH_SAPLING,
            Self::Jungle | Self::MegaJungle => JUNGLE_SAPLING,
            Self::Spruce | Self::Pine | Self::MegaSpruce | Self::MegaPine => SPRUCE_SAPLING,
            Self::Oak | Self::FancyOak | Self::SwampOak | Self::JungleBush | Self::Azalea => OAK_SAPLING,
        }
    }

    fn grow(self, table: &mut BlockTable) -> Tree {
        match self {
            Self::Oak => ignoring_vines(straight_blob(table, OAK_LOG, OAK_LEAVES, 4, 2, 0, 2)),
            Self::FancyOak => fancy_oak(table),
            Self::SwampOak => {
                let decorator = leave_vine(table, 0.25);
                with_decorators(straight_blob(table, OAK_LOG, OAK_LEAVES, 5, 3, 0, 3), vec![decorator])
            }
            Self::Birch => ignoring_vines(straight_blob(table, BIRCH_LOG, BIRCH_LEAVES, 5, 2, 0, 2)),
            Self::SuperBirch => ignoring_vines(straight_blob(table, BIRCH_LOG, BIRCH_LEAVES, 5, 2, 6, 2)),
            Self::DarkOak => dark_oak_like(table, DARK_OAK_LOG, DARK_OAK_LEAVES),
            Self::PaleOak => {
                let decorator = pale_moss(table);
                with_decorators(dark_oak_like(table, PALE_OAK_LOG, PALE_OAK_LEAVES), vec![decorator])
            }
            Self::CreakingPaleOak => {
                let decorators = vec![
                    pale_moss(table),
                    Decorator::CreakingHeart {
                        probability: 1.0,
                        heart: table.get(CREAKING_HEART, &[state(CREAKING_HEART_STATE, text("dormant")), state(NATURAL, flag(true))]),
                    },
                ];
                with_decorators(dark_oak_like(table, PALE_OAK_LOG, PALE_OAK_LEAVES), decorators)
            }
            Self::Acacia => {
                let foliage_state = leaves(table, ACACIA_LEAVES).into();
                ignoring_vines(tree(
                    table,
                    ACACIA_LOG,
                    trunk(5, 2, 2, TrunkKind::Forking),
                    foliage_state,
                    foliage(2, 0, FoliageKind::Acacia),
                    FeatureSize::two(1, 0, 2),
                ))
            }
            Self::Cherry => cherry(table),
            Self::Spruce => {
                let foliage_state = leaves(table, SPRUCE_LEAVES).into();
                let placer = foliage(
                    IntProvider::Uniform(2, 3),
                    IntProvider::Uniform(0, 2),
                    FoliageKind::Spruce {
                        trunk_height: IntProvider::Uniform(1, 2),
                    },
                );
                ignoring_vines(tree(table, SPRUCE_LOG, trunk(5, 2, 1, TrunkKind::Straight), foliage_state, placer, FeatureSize::two(2, 0, 2)))
            }
            Self::Pine => {
                let foliage_state = leaves(table, SPRUCE_LEAVES).into();
                let placer = foliage(1, 1, FoliageKind::Pine { height: IntProvider::Uniform(3, 4) });
                ignoring_vines(tree(table, SPRUCE_LOG, trunk(6, 4, 0, TrunkKind::Straight), foliage_state, placer, FeatureSize::two(2, 0, 2)))
            }
            Self::MegaSpruce => mega_conifer(table, IntProvider::Uniform(13, 17)),
            Self::MegaPine => mega_conifer(table, IntProvider::Uniform(3, 7)),
            Self::Jungle => {
                let decorators = vec![cocoa(table, 0.2), trunk_vine(table), leave_vine(table, 0.25)];
                ignoring_vines(with_decorators(jungle(table), decorators))
            }
            Self::MegaJungle => {
                let foliage_state = leaves(table, JUNGLE_LEAVES).into();
                let tree = tree(
                    table,
                    JUNGLE_LOG,
                    trunk(10, 2, 19, TrunkKind::MegaJungle),
                    foliage_state,
                    foliage(2, 0, FoliageKind::MegaJungle { height: 2 }),
                    FeatureSize::two(1, 1, 2),
                );
                let decorators = vec![trunk_vine(table), leave_vine(table, 0.25)];
                with_decorators(tree, decorators)
            }
            Self::JungleBush => {
                let foliage_state = leaves(table, OAK_LEAVES).into();
                tree(
                    table,
                    JUNGLE_LOG,
                    trunk(1, 0, 0, TrunkKind::Straight),
                    foliage_state,
                    foliage(2, 1, FoliageKind::Bush { height: 2 }),
                    FeatureSize::two(0, 0, 0),
                )
            }
            Self::Azalea => {
                let foliage_state = StateProvider::Weighted(vec![(leaves(table, AZALEA_LEAVES), 3), (leaves(table, AZALEA_LEAVES_FLOWERED), 1)]);
                let kind = TrunkKind::Bending {
                    min_height_for_leaves: 3,
                    bend_length: IntProvider::Uniform(1, 2),
                };
                let placer = foliage(3, 0, FoliageKind::RandomSpread { height: 2.into(), attempts: 50 });
                let mut tree = tree(table, OAK_LOG, trunk(4, 2, 0, kind), foliage_state, placer, FeatureSize::two(1, 0, 1));
                tree.below_trunk = table.name(DIRT_WITH_ROOTS).into();
                tree
            }
            Self::Mangrove => mangrove(table, 2, 4, 4, (1, 3), 2),
            Self::TallMangrove => mangrove(table, 4, 9, 6, (3, 7), 3),
            Self::RedPoplar => poplar(table, RED_POPLAR_LEAVES),
            Self::OrangePoplar => poplar(table, ORANGE_POPLAR_LEAVES),
            Self::YellowPoplar => poplar(table, YELLOW_POPLAR_LEAVES),
        }
    }
}

impl Standing {
    pub const fn with_leaf_litter(self) -> Self {
        Self { leaf_litter: true, ..self }
    }

    fn grow(self, table: &mut BlockTable) -> Tree {
        let mut decorators = Vec::new();
        if let Some(chance) = self.bees {
            decorators.push(beehive(table, chance));
        }
        if self.leaf_litter {
            decorators.extend(leaf_litter(table));
        }
        with_decorators(self.species.grow(table), decorators)
    }
}

impl Fallen {
    fn sapling(self) -> &'static str {
        match self {
            Self::Oak => OAK_SAPLING,
            Self::Birch | Self::SuperBirch => BIRCH_SAPLING,
            Self::Jungle => JUNGLE_SAPLING,
            Self::Spruce => SPRUCE_SAPLING,
            Self::Poplar => POPLAR_SAPLING,
        }
    }

    fn grow(self, table: &mut BlockTable) -> FallenTree {
        match self {
            Self::Oak => fallen_log(table, OAK_LOG, 4, 7, true),
            Self::Birch => fallen_log(table, BIRCH_LOG, 5, 8, false),
            Self::SuperBirch => fallen_log(table, BIRCH_LOG, 5, 15, false),
            Self::Jungle => fallen_log(table, JUNGLE_LOG, 4, 11, true),
            Self::Spruce => fallen_log(table, SPRUCE_LOG, 6, 10, false),
            Self::Poplar => {
                let mut fallen = fallen_log(table, POPLAR_LOG, 4, 7, false);
                fallen.log_decorators = vec![
                    Decorator::AttachedToLogs {
                        probability: 0.1,
                        provider: table.name(BROWN_MUSHROOM).into(),
                        directions: vec![Direction::Up],
                    },
                    shelf_mushroom(table, 0.8),
                ];
                fallen
            }
        }
    }
}

impl TreeFeature {
    fn sapling(self) -> &'static str {
        match self {
            Self::Standing(standing) => standing.species.sapling(),
            Self::Fallen(fallen) => fallen.sapling(),
            Self::HugeBrownMushroom | Self::HugeRedMushroom => OAK_SAPLING,
        }
    }

    pub fn feature(self, table: &mut BlockTable) -> Feature {
        match self {
            Self::Standing(standing) => Feature::Tree(Box::new(standing.grow(table))),
            Self::Fallen(fallen) => Feature::FallenTree(fallen.grow(table)),
            Self::HugeBrownMushroom => Feature::HugeMushroom(huge_mushroom(table, false)),
            Self::HugeRedMushroom => Feature::HugeMushroom(huge_mushroom(table, true)),
        }
    }
}

impl From<Species> for TreeFeature {
    fn from(species: Species) -> Self {
        Self::Standing(Standing {
            species,
            bees: None,
            leaf_litter: false,
        })
    }
}

impl From<Standing> for TreeFeature {
    fn from(standing: Standing) -> Self {
        Self::Standing(standing)
    }
}

impl From<Fallen> for TreeFeature {
    fn from(fallen: Fallen) -> Self {
        Self::Fallen(fallen)
    }
}

fn mega_conifer(table: &mut BlockTable, crown_height: IntProvider) -> Tree {
    let foliage_state = leaves(table, SPRUCE_LEAVES).into();
    let tree = tree(
        table,
        SPRUCE_LOG,
        trunk(13, 2, 14, TrunkKind::Giant),
        foliage_state,
        foliage(0, 0, FoliageKind::MegaPine { crown_height }),
        FeatureSize::two(1, 1, 2),
    );
    let decorator = alter_ground_podzol(table);
    with_decorators(tree, vec![decorator])
}

pub fn checked(table: &mut BlockTable, tree: impl Into<TreeFeature>) -> Placed {
    let tree = tree.into();
    let sapling = table.name(tree.sapling());
    Placed::new(tree.feature(table), vec![filter(BlockPredicate::WouldSurvive(IVec3::ZERO, sapling))])
}

pub fn on_snow(table: &mut BlockTable, tree: impl Into<TreeFeature>) -> Placed {
    let powder = table.kind(POWDER_SNOW);
    let snow = table.kind(SNOW);
    let scan = super::placement::Modifier::EnvironmentScan {
        direction: Direction::Up,
        target: BlockPredicate::MatchingBlocks(IVec3::ZERO, vec![powder]).not(),
        allowed: BlockPredicate::True,
        max_steps: 8,
    };
    Placed::new(tree.into().feature(table), vec![scan, filter(BlockPredicate::MatchingBlocks(IVec3::NEG_Y, vec![snow, powder]))])
}

pub fn inline(table: &mut BlockTable, tree: impl Into<TreeFeature>) -> Placed {
    Placed::new(tree.into().feature(table), Vec::new())
}
