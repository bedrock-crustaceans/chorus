mod aquatic;
mod basic;
mod biome_features;
mod caves;
mod key;
mod misc;
mod multiface;
mod ore;
mod placement;
mod predicate;
mod provider;
mod sculk;
mod speleothem;
mod tree;
mod trees;
mod vegetation;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use glam::IVec3;

use super::biome::Biome;
use super::blocks::{BlockTable, Blocks};
use super::proto::ProtoChunk;
use super::random::{RandomSource, WorldgenRandom, Xoroshiro};
use biome_features::STEP_COUNT;
pub use key::Key;
use placement::Placed;

use super::{region, shape};
pub use region::{BlockView, Changes, Region};
pub use shape::Shapes;

pub const SEA_LEVEL: i32 = super::SEA_LEVEL;

pub enum Feature {
    NoOp,
    Ore(ore::Ore),
    SimpleBlock(basic::SimpleBlock),
    Sequence(Vec<Placed>),
    RandomSelector { features: Vec<(Placed, f32)>, default: Box<Placed> },
    SimpleRandomSelector(Vec<Placed>),
    WeightedSelector(Vec<(Placed, i32)>),
    RandomBoolean(Box<Placed>, Box<Placed>),
    Spike(misc::Spike),
    Disk(misc::Disk),
    BlockBlob(misc::BlockBlob),
    BlueIce(misc::BlueIce),
    Lake(misc::Lake),
    SnowAndFreeze(misc::SnowAndFreeze),
    Spring(misc::Spring),
    Iceberg(misc::Iceberg),
    Tree(Box<tree::Tree>),
    FallenTree(tree::FallenTree),
    HugeMushroom(tree::HugeMushroom),
    BlockColumn(vegetation::BlockColumn),
    Bamboo(vegetation::Bamboo),
    Vines(vegetation::Vines),
    VegetationPatch(vegetation::VegetationPatch),
    Overlay(Vec<Placed>),
    CoralTree(Box<Placed>),
    CoralClaw(Box<Placed>),
    MonsterRoom(caves::MonsterRoom),
    Speleothem(speleothem::Speleothem),
    SpeleothemCluster(speleothem::SpeleothemCluster),
    LargeDripstone(speleothem::LargeDripstone),
    UnderwaterMagma(caves::UnderwaterMagma),
    MultifaceGrowth(multiface::MultifaceGrowth),
    RootSystem(caves::RootSystem),
    Geode(caves::Geode),
    SculkPatch(Box<sculk::SculkPatch>),
}

impl Feature {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        match self {
            Self::NoOp => false,
            Self::Ore(ore) => ore.place(ctx, random, origin),
            Self::SimpleBlock(feature) => feature.place(ctx, random, origin),
            Self::Sequence(features) => basic::sequence(ctx, random, features, origin),
            Self::RandomSelector { features, default } => basic::random_selector(ctx, random, features, default, origin),
            Self::SimpleRandomSelector(features) => basic::simple_random_selector(ctx, random, features, origin),
            Self::WeightedSelector(features) => basic::weighted_selector(ctx, random, features, origin),
            Self::RandomBoolean(first, second) => {
                if random.next_boolean() {
                    first.place(ctx, random, origin, None)
                } else {
                    second.place(ctx, random, origin, None)
                }
            }
            Self::Spike(feature) => feature.place(ctx, random, origin),
            Self::Disk(feature) => feature.place(ctx, random, origin),
            Self::BlockBlob(feature) => feature.place(ctx, random, origin),
            Self::BlueIce(feature) => feature.place(ctx, random, origin),
            Self::Lake(feature) => feature.place(ctx, random, origin),
            Self::SnowAndFreeze(feature) => feature.place(ctx, origin),
            Self::Spring(feature) => feature.place(ctx, origin),
            Self::Iceberg(feature) => feature.place(ctx, random, origin),
            Self::Tree(feature) => feature.place(ctx, random, origin),
            Self::FallenTree(feature) => feature.place(ctx, random, origin),
            Self::HugeMushroom(feature) => feature.place(ctx, random, origin),
            Self::BlockColumn(feature) => feature.place(ctx, random, origin),
            Self::Bamboo(feature) => feature.place(ctx, random, origin),
            Self::Vines(feature) => feature.place(ctx, origin),
            Self::VegetationPatch(feature) => feature.place(ctx, random, origin),
            Self::Overlay(features) => features.iter().fold(false, |placed, feature| feature.place(ctx, random, origin, None) | placed),
            Self::CoralTree(feature) => aquatic::coral_tree(ctx, random, feature, origin),
            Self::CoralClaw(feature) => aquatic::coral_claw(ctx, random, feature, origin),
            Self::MonsterRoom(feature) => feature.place(ctx, random, origin),
            Self::Speleothem(feature) => feature.place(ctx, random, origin),
            Self::SpeleothemCluster(feature) => feature.place(ctx, random, origin),
            Self::LargeDripstone(feature) => feature.place(ctx, random, origin),
            Self::UnderwaterMagma(feature) => feature.place(ctx, random, origin),
            Self::MultifaceGrowth(feature) => feature.place(ctx, random, origin),
            Self::RootSystem(feature) => feature.place(ctx, random, origin),
            Self::Geode(feature) => feature.place(ctx, random, origin),
            Self::SculkPatch(feature) => feature.place(ctx, random, origin),
        }
    }
}

pub struct Context<'a, 'b> {
    pub region: &'a mut Region<'b>,
    pub catalog: &'a Catalog,
    pub region_random: &'a mut Xoroshiro,
}

pub struct Catalog {
    pub moss_carpet: super::moss_carpet::MossCarpet,
    placed: HashMap<Key, Placed>,
    membership: HashMap<Biome, [Vec<Key>; STEP_COUNT]>,
    order: [Vec<Key>; STEP_COUNT],
}

fn define(key: Key, table: &mut BlockTable, seed: i64) -> Placed {
    ore::define(key, table)
        .or_else(|| misc::define(key, table))
        .or_else(|| vegetation::define(key, table))
        .or_else(|| aquatic::define(key, table))
        .or_else(|| caves::define(key, table, seed))
        .unwrap_or_else(|| Placed::new(Feature::NoOp, Vec::new()))
}

impl Catalog {
    pub fn new(table: &mut BlockTable, seed: i64) -> Self {
        let placed = Key::ALL.iter().map(|&key| (key, define(key, table, seed))).collect();
        let membership: HashMap<Biome, [Vec<Key>; STEP_COUNT]> = Biome::ALL.iter().map(|&biome| (biome, biome_features::features(biome))).collect();
        let order = Self::sort(&membership);
        Self {
            moss_carpet: super::moss_carpet::MossCarpet::register(table),
            placed,
            membership,
            order,
        }
    }

    fn sort(membership: &HashMap<Biome, [Vec<Key>; STEP_COUNT]>) -> [Vec<Key>; STEP_COUNT] {
        let mut index: HashMap<Key, usize> = HashMap::new();
        let mut edges: BTreeMap<(usize, usize), BTreeSet<(usize, usize)>> = BTreeMap::new();
        let mut keys: HashMap<(usize, usize), Key> = HashMap::new();
        for biome in Biome::ALL {
            let mut list = Vec::new();
            for (step, features) in membership[&biome].iter().enumerate() {
                for &key in features {
                    let next = index.len();
                    let feature_index = *index.entry(key).or_insert(next);
                    keys.insert((step, feature_index), key);
                    list.push((step, feature_index));
                }
            }
            for (i, &node) in list.iter().enumerate() {
                let targets = edges.entry(node).or_default();
                if let Some(&next) = list.get(i + 1) {
                    targets.insert(next);
                }
            }
        }

        fn visit(
            node: (usize, usize),
            edges: &BTreeMap<(usize, usize), BTreeSet<(usize, usize)>>,
            discovered: &mut BTreeSet<(usize, usize)>,
            visiting: &mut BTreeSet<(usize, usize)>,
            out: &mut Vec<(usize, usize)>,
        ) {
            if discovered.contains(&node) || visiting.contains(&node) {
                return;
            }
            visiting.insert(node);
            if let Some(targets) = edges.get(&node) {
                for &target in targets {
                    visit(target, edges, discovered, visiting, out);
                }
            }
            visiting.remove(&node);
            discovered.insert(node);
            out.push(node);
        }

        let mut discovered = BTreeSet::new();
        let mut visiting = BTreeSet::new();
        let mut sorted = Vec::new();
        for &node in edges.keys() {
            visit(node, &edges, &mut discovered, &mut visiting, &mut sorted);
        }
        sorted.reverse();
        std::array::from_fn(|step| sorted.iter().filter(|(s, _)| *s == step).map(|node| keys[node]).collect())
    }

    pub fn biome_has(&self, biome: Biome, key: Key) -> bool {
        self.membership.get(&biome).is_some_and(|steps| steps.iter().any(|features| features.contains(&key)))
    }

    pub fn decorate(&self, seed: i64, owner_x: i32, owner_z: i32, blocks: &Blocks, chunks: [ProtoChunk; 9]) -> Changes {
        let mut region = Region::new(owner_x, owner_z, blocks, chunks);
        let possible = region.biomes();
        let origin = IVec3::new(owner_x << 4, region.min_y(), owner_z << 4);
        let mut random = WorldgenRandom::new(Xoroshiro::new(0));
        let decoration_seed = random.set_decoration_seed(seed, owner_x << 4, owner_z << 4);
        let mut region_random = Xoroshiro::new(seed)
            .fork_positional()
            .hash_of("minecraft:worldgen_region_random")
            .fork_positional()
            .at(owner_x << 4, 0, owner_z << 4);
        let mut ctx = Context {
            region: &mut region,
            catalog: self,
            region_random: &mut region_random,
        };
        for step in 0..STEP_COUNT {
            for (index, &key) in self.order[step].iter().enumerate() {
                if !possible.iter().any(|&biome| self.membership[&biome][step].contains(&key)) {
                    continue;
                }
                random.set_feature_seed(decoration_seed, index as i32, step as i32);
                self.placed[&key].place(&mut ctx, &mut random, origin, Some(key));
            }
        }
        region.into_changes()
    }
}
