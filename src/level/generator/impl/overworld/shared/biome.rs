use std::sync::LazyLock;

use super::biome_builder::{OverworldBiomeBuilder, ParameterPoint};
use super::noise::SimplexNoise;
use super::random::Legacy;
use crate::level::biome::biome_id::BiomeID;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Biome {
    Ocean,
    DeepOcean,
    FrozenOcean,
    DeepFrozenOcean,
    ColdOcean,
    DeepColdOcean,
    LukewarmOcean,
    DeepLukewarmOcean,
    WarmOcean,
    MushroomFields,
    Plains,
    SunflowerPlains,
    SnowyPlains,
    IceSpikes,
    Desert,
    Swamp,
    MangroveSwamp,
    Forest,
    FlowerForest,
    BirchForest,
    DarkForest,
    PaleGarden,
    OldGrowthBirchForest,
    OldGrowthPineTaiga,
    OldGrowthSpruceTaiga,
    Taiga,
    SnowyTaiga,
    Savanna,
    SavannaPlateau,
    WindsweptHills,
    WindsweptGravellyHills,
    WindsweptForest,
    WindsweptSavanna,
    Jungle,
    SparseJungle,
    BambooJungle,
    Badlands,
    ErodedBadlands,
    WoodedBadlands,
    Meadow,
    CherryGrove,
    Grove,
    SnowySlopes,
    FrozenPeaks,
    JaggedPeaks,
    StonyPeaks,
    River,
    FrozenRiver,
    Beach,
    SnowyBeach,
    StonyShore,
    DripstoneCaves,
    LushCaves,
    DeepDark,
    DappledForest,
    SulfurCaves,
}

impl Biome {
    pub const ALL: [Biome; 56] = [
        Self::Ocean,
        Self::DeepOcean,
        Self::FrozenOcean,
        Self::DeepFrozenOcean,
        Self::ColdOcean,
        Self::DeepColdOcean,
        Self::LukewarmOcean,
        Self::DeepLukewarmOcean,
        Self::WarmOcean,
        Self::MushroomFields,
        Self::Plains,
        Self::SunflowerPlains,
        Self::SnowyPlains,
        Self::IceSpikes,
        Self::Desert,
        Self::Swamp,
        Self::MangroveSwamp,
        Self::Forest,
        Self::FlowerForest,
        Self::BirchForest,
        Self::DarkForest,
        Self::PaleGarden,
        Self::OldGrowthBirchForest,
        Self::OldGrowthPineTaiga,
        Self::OldGrowthSpruceTaiga,
        Self::Taiga,
        Self::SnowyTaiga,
        Self::Savanna,
        Self::SavannaPlateau,
        Self::WindsweptHills,
        Self::WindsweptGravellyHills,
        Self::WindsweptForest,
        Self::WindsweptSavanna,
        Self::Jungle,
        Self::SparseJungle,
        Self::BambooJungle,
        Self::Badlands,
        Self::ErodedBadlands,
        Self::WoodedBadlands,
        Self::Meadow,
        Self::CherryGrove,
        Self::Grove,
        Self::SnowySlopes,
        Self::FrozenPeaks,
        Self::JaggedPeaks,
        Self::StonyPeaks,
        Self::River,
        Self::FrozenRiver,
        Self::Beach,
        Self::SnowyBeach,
        Self::StonyShore,
        Self::DripstoneCaves,
        Self::LushCaves,
        Self::DeepDark,
        Self::DappledForest,
        Self::SulfurCaves,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Ocean => "ocean",
            Self::DeepOcean => "deep_ocean",
            Self::FrozenOcean => "frozen_ocean",
            Self::DeepFrozenOcean => "deep_frozen_ocean",
            Self::ColdOcean => "cold_ocean",
            Self::DeepColdOcean => "deep_cold_ocean",
            Self::LukewarmOcean => "lukewarm_ocean",
            Self::DeepLukewarmOcean => "deep_lukewarm_ocean",
            Self::WarmOcean => "warm_ocean",
            Self::MushroomFields => "mushroom_fields",
            Self::Plains => "plains",
            Self::SunflowerPlains => "sunflower_plains",
            Self::SnowyPlains => "snowy_plains",
            Self::IceSpikes => "ice_spikes",
            Self::Desert => "desert",
            Self::Swamp => "swamp",
            Self::MangroveSwamp => "mangrove_swamp",
            Self::Forest => "forest",
            Self::FlowerForest => "flower_forest",
            Self::BirchForest => "birch_forest",
            Self::DarkForest => "dark_forest",
            Self::PaleGarden => "pale_garden",
            Self::OldGrowthBirchForest => "old_growth_birch_forest",
            Self::OldGrowthPineTaiga => "old_growth_pine_taiga",
            Self::OldGrowthSpruceTaiga => "old_growth_spruce_taiga",
            Self::Taiga => "taiga",
            Self::SnowyTaiga => "snowy_taiga",
            Self::Savanna => "savanna",
            Self::SavannaPlateau => "savanna_plateau",
            Self::WindsweptHills => "windswept_hills",
            Self::WindsweptGravellyHills => "windswept_gravelly_hills",
            Self::WindsweptForest => "windswept_forest",
            Self::WindsweptSavanna => "windswept_savanna",
            Self::Jungle => "jungle",
            Self::SparseJungle => "sparse_jungle",
            Self::BambooJungle => "bamboo_jungle",
            Self::Badlands => "badlands",
            Self::ErodedBadlands => "eroded_badlands",
            Self::WoodedBadlands => "wooded_badlands",
            Self::Meadow => "meadow",
            Self::CherryGrove => "cherry_grove",
            Self::Grove => "grove",
            Self::SnowySlopes => "snowy_slopes",
            Self::FrozenPeaks => "frozen_peaks",
            Self::JaggedPeaks => "jagged_peaks",
            Self::StonyPeaks => "stony_peaks",
            Self::River => "river",
            Self::FrozenRiver => "frozen_river",
            Self::Beach => "beach",
            Self::SnowyBeach => "snowy_beach",
            Self::StonyShore => "stony_shore",
            Self::DripstoneCaves => "dripstone_caves",
            Self::LushCaves => "lush_caves",
            Self::DeepDark => "deep_dark",
            Self::DappledForest => "dappled_forest",
            Self::SulfurCaves => "sulfur_caves",
        }
    }

    pub fn bedrock_id(self) -> i32 {
        match self {
            Self::Ocean => BiomeID::OCEAN,
            Self::DeepOcean => BiomeID::DEEP_OCEAN,
            Self::FrozenOcean => BiomeID::FROZEN_OCEAN,
            Self::DeepFrozenOcean => BiomeID::DEEP_FROZEN_OCEAN,
            Self::ColdOcean => BiomeID::COLD_OCEAN,
            Self::DeepColdOcean => BiomeID::DEEP_COLD_OCEAN,
            Self::LukewarmOcean => BiomeID::LUKEWARM_OCEAN,
            Self::DeepLukewarmOcean => BiomeID::DEEP_LUKEWARM_OCEAN,
            Self::WarmOcean => BiomeID::WARM_OCEAN,
            Self::MushroomFields => BiomeID::MUSHROOM_ISLAND,
            Self::Plains => BiomeID::PLAINS,
            Self::SunflowerPlains => BiomeID::SUNFLOWER_PLAINS,
            Self::SnowyPlains => BiomeID::ICE_PLAINS,
            Self::IceSpikes => BiomeID::ICE_PLAINS_SPIKES,
            Self::Desert => BiomeID::DESERT,
            Self::Swamp => BiomeID::SWAMPLAND,
            Self::MangroveSwamp => BiomeID::MANGROVE_SWAMP,
            Self::Forest => BiomeID::FOREST,
            Self::FlowerForest => BiomeID::FLOWER_FOREST,
            Self::BirchForest => BiomeID::BIRCH_FOREST,
            Self::DarkForest => BiomeID::ROOFED_FOREST,
            Self::PaleGarden => BiomeID::PALE_GARDEN,
            Self::OldGrowthBirchForest => BiomeID::BIRCH_FOREST_MUTATED,
            Self::OldGrowthPineTaiga => BiomeID::MEGA_TAIGA,
            Self::OldGrowthSpruceTaiga => BiomeID::REDWOOD_TAIGA_MUTATED,
            Self::Taiga => BiomeID::TAIGA,
            Self::SnowyTaiga => BiomeID::COLD_TAIGA,
            Self::Savanna => BiomeID::SAVANNA,
            Self::SavannaPlateau => BiomeID::SAVANNA_PLATEAU,
            Self::WindsweptHills => BiomeID::EXTREME_HILLS,
            Self::WindsweptGravellyHills => BiomeID::EXTREME_HILLS_MUTATED,
            Self::WindsweptForest => BiomeID::EXTREME_HILLS_PLUS_TREES,
            Self::WindsweptSavanna => BiomeID::SAVANNA_MUTATED,
            Self::Jungle => BiomeID::JUNGLE,
            Self::SparseJungle => BiomeID::JUNGLE_EDGE,
            Self::BambooJungle => BiomeID::BAMBOO_JUNGLE,
            Self::Badlands => BiomeID::MESA,
            Self::ErodedBadlands => BiomeID::MESA_BRYCE,
            Self::WoodedBadlands => BiomeID::MESA_PLATEAU_STONE,
            Self::Meadow => BiomeID::MEADOW,
            Self::CherryGrove => BiomeID::CHERRY_GROVE,
            Self::Grove => BiomeID::GROVE,
            Self::SnowySlopes => BiomeID::SNOWY_SLOPES,
            Self::FrozenPeaks => BiomeID::FROZEN_PEAKS,
            Self::JaggedPeaks => BiomeID::JAGGED_PEAKS,
            Self::StonyPeaks => BiomeID::STONY_PEAKS,
            Self::River => BiomeID::RIVER,
            Self::FrozenRiver => BiomeID::FROZEN_RIVER,
            Self::Beach => BiomeID::BEACH,
            Self::SnowyBeach => BiomeID::COLD_BEACH,
            Self::StonyShore => BiomeID::STONE_BEACH,
            Self::DripstoneCaves => BiomeID::DRIPSTONE_CAVES,
            Self::LushCaves => BiomeID::LUSH_CAVES,
            Self::DeepDark => BiomeID::DEEP_DARK,
            Self::DappledForest => BiomeID::DAPPLED_FOREST,
            Self::SulfurCaves => BiomeID::SULFUR_CAVES,
        }
    }

    fn base_temperature(self) -> f32 {
        match self {
            Self::Ocean => 0.5,
            Self::DeepOcean => 0.5,
            Self::FrozenOcean => 0.0,
            Self::DeepFrozenOcean => 0.5,
            Self::ColdOcean => 0.5,
            Self::DeepColdOcean => 0.5,
            Self::LukewarmOcean => 0.5,
            Self::DeepLukewarmOcean => 0.5,
            Self::WarmOcean => 0.5,
            Self::MushroomFields => 0.9,
            Self::Plains => 0.8,
            Self::SunflowerPlains => 0.8,
            Self::SnowyPlains => 0.0,
            Self::IceSpikes => 0.0,
            Self::Desert => 2.0,
            Self::Swamp => 0.8,
            Self::MangroveSwamp => 0.8,
            Self::Forest => 0.7,
            Self::FlowerForest => 0.7,
            Self::BirchForest => 0.6,
            Self::DarkForest => 0.7,
            Self::PaleGarden => 0.7,
            Self::OldGrowthBirchForest => 0.6,
            Self::OldGrowthPineTaiga => 0.3,
            Self::OldGrowthSpruceTaiga => 0.25,
            Self::Taiga => 0.25,
            Self::SnowyTaiga => -0.5,
            Self::Savanna => 2.0,
            Self::SavannaPlateau => 2.0,
            Self::WindsweptHills => 0.2,
            Self::WindsweptGravellyHills => 0.2,
            Self::WindsweptForest => 0.2,
            Self::WindsweptSavanna => 2.0,
            Self::Jungle => 0.95,
            Self::SparseJungle => 0.95,
            Self::BambooJungle => 0.95,
            Self::Badlands => 2.0,
            Self::ErodedBadlands => 2.0,
            Self::WoodedBadlands => 2.0,
            Self::Meadow => 0.5,
            Self::CherryGrove => 0.5,
            Self::Grove => -0.2,
            Self::SnowySlopes => -0.3,
            Self::FrozenPeaks => -0.7,
            Self::JaggedPeaks => -0.7,
            Self::StonyPeaks => 1.0,
            Self::River => 0.5,
            Self::FrozenRiver => 0.0,
            Self::Beach => 0.8,
            Self::SnowyBeach => 0.05,
            Self::StonyShore => 0.2,
            Self::DripstoneCaves => 0.8,
            Self::LushCaves => 0.5,
            Self::DeepDark => 0.8,
            Self::DappledForest => 0.6,
            Self::SulfurCaves => 0.8,
        }
    }

    pub fn has_precipitation(self) -> bool {
        !matches!(
            self,
            Self::Desert | Self::Savanna | Self::SavannaPlateau | Self::WindsweptSavanna | Self::Badlands | Self::ErodedBadlands | Self::WoodedBadlands
        )
    }

    fn frozen(self) -> bool {
        matches!(self, Self::FrozenOcean | Self::DeepFrozenOcean)
    }

    pub fn temperature_at(self, x: i32, y: i32, z: i32, sea_level: i32) -> f32 {
        let mut temperature = self.base_temperature();
        if self.frozen() {
            let large = FROZEN_TEMPERATURE_NOISE.get(x as f64 * 0.05, z as f64 * 0.05) as f64 * 7.0;
            let edge = BIOME_INFO_NOISE.get(x as f64 * 0.2, z as f64 * 0.2) as f64;
            if large + edge < 0.3 && (BIOME_INFO_NOISE.get(x as f64 * 0.09, z as f64 * 0.09) as f64) < 0.8 {
                temperature = 0.2;
            }
        }
        let snow_level = sea_level + 17;
        if y > snow_level {
            let v = TEMPERATURE_NOISE.get((x as f32 / 8.0) as f64, (z as f32 / 8.0) as f64) * 8.0;
            return temperature - (v + y as f32 - snow_level as f32) * 0.05 / 40.0;
        }
        temperature
    }

    pub fn warm_enough_to_rain(self, x: i32, y: i32, z: i32, sea_level: i32) -> bool {
        self.temperature_at(x, y, z, sea_level) >= 0.15
    }

    pub fn cold_enough_to_snow(self, x: i32, y: i32, z: i32, sea_level: i32) -> bool {
        self.temperature_at(x, y, z, sea_level) < 0.15
    }
}

struct FrozenNoise([SimplexNoise; 3]);

impl FrozenNoise {
    fn get(&self, x: f64, z: f64) -> f32 {
        let [a, b, c] = &self.0;
        0.14285715 * a.get(x, z) + 0.2857143 * b.get(x * 0.5, z * 0.5) + 0.5714286 * c.get(x * 0.25, z * 0.25)
    }
}

static TEMPERATURE_NOISE: LazyLock<SimplexNoise> = LazyLock::new(|| SimplexNoise::without_offset(&mut Legacy::new(1234)));
pub static BIOME_INFO_NOISE: LazyLock<SimplexNoise> = LazyLock::new(|| SimplexNoise::without_offset(&mut Legacy::new(2345)));
static FROZEN_TEMPERATURE_NOISE: LazyLock<FrozenNoise> = LazyLock::new(|| {
    let mut random = Legacy::new(3456);
    FrozenNoise([
        SimplexNoise::without_offset(&mut random),
        SimplexNoise::without_offset(&mut random),
        SimplexNoise::without_offset(&mut random),
    ])
});

const CHILDREN_PER_NODE: usize = 19;

type ParameterSpace = [(i64, i64); 7];

fn parameter_distance((min, max): (i64, i64), target: i64) -> i64 {
    let above = target - max;
    let below = min - target;
    if above > 0 { above } else { below.max(0) }
}

#[cfg(test)]
fn space_distance(space: &ParameterSpace, target: &[i64; 7]) -> i64 {
    space.iter().zip(target).map(|(&parameter, &t)| parameter_distance(parameter, t).pow(2)).sum()
}

#[cfg(test)]
const SELECTIVE_ORDER: [usize; 7] = [4, 2, 3, 0, 1, 5, 6];

#[cfg(test)]
fn distance_below(space: &ParameterSpace, target: &[i64; 7], limit: i64) -> Option<i64> {
    let mut distance = 0;
    for dimension in SELECTIVE_ORDER {
        distance += parameter_distance(space[dimension], target[dimension]).pow(2);
        if distance > limit {
            return None;
        }
    }
    Some(distance)
}

const DEPTH: usize = 4;
const FIXED_ORDER: [usize; 6] = [2, 3, 0, 1, 5, 6];

fn fixed_distance_below(space: &ParameterSpace, target: &[i64; 7], limit: i64) -> Option<i64> {
    let mut distance = 0;
    for dimension in FIXED_ORDER {
        distance += parameter_distance(space[dimension], target[dimension]).pow(2);
        if distance > limit {
            return None;
        }
    }
    Some(distance)
}

#[derive(Clone, Copy)]
pub struct Nearest {
    distance: i64,
    order: usize,
    biome: Biome,
    space: ParameterSpace,
}

enum Node {
    Leaf { space: ParameterSpace, biome: Biome, order: usize },
    SubTree { space: ParameterSpace, children: Vec<Node> },
}

impl Node {
    fn assign_order(&mut self, next: &mut usize) {
        match self {
            Self::Leaf { order, .. } => {
                *order = *next;
                *next += 1;
            }
            Self::SubTree { children, .. } => children.iter_mut().for_each(|child| child.assign_order(next)),
        }
    }

    fn collect_leaves(&self, out: &mut Vec<Node>) {
        match self {
            &Self::Leaf { space, biome, order } => out.push(Self::Leaf { space, biome, order }),
            Self::SubTree { children, .. } => children.iter().for_each(|child| child.collect_leaves(out)),
        }
    }

    fn nearest_fixed(&self, target: &[i64; 7], best: &mut Option<Nearest>) {
        let limit = best.map_or(i64::MAX, |best| best.distance);
        let Some(distance) = fixed_distance_below(self.space(), target, limit) else { return };
        match self {
            &Self::Leaf { space, biome, order } => {
                if best.is_none_or(|best| distance < best.distance || (distance == best.distance && order < best.order)) {
                    *best = Some(Nearest { distance, order, biome, space });
                }
            }
            Self::SubTree { children, .. } => children.iter().for_each(|child| child.nearest_fixed(target, best)),
        }
    }

    fn space(&self) -> &ParameterSpace {
        match self {
            Self::Leaf { space, .. } | Self::SubTree { space, .. } => space,
        }
    }

    fn subtree(children: Vec<Node>) -> Self {
        let mut space = *children[0].space();
        for child in &children[1..] {
            for (bound, &(min, max)) in space.iter_mut().zip(child.space()) {
                *bound = (bound.0.min(min), bound.1.max(max));
            }
        }
        Self::SubTree { space, children }
    }

    fn build(mut children: Vec<Node>) -> Self {
        if children.len() == 1 {
            return children.pop().unwrap();
        }
        if children.len() <= CHILDREN_PER_NODE {
            children.sort_by_key(|child| child.space().iter().map(|&(min, max)| ((min + max) / 2).abs()).sum::<i64>());
            return Self::subtree(children);
        }
        let mut best: Option<(i64, usize)> = None;
        for dimension in 0..7 {
            sort_nodes(&mut children, dimension, false);
            let cost: i64 = bucket_spaces(&children).iter().map(|space| space.iter().map(|&(min, max)| (max - min).abs()).sum::<i64>()).sum();
            if best.is_none_or(|(min_cost, _)| min_cost > cost) {
                best = Some((cost, dimension));
            }
        }
        let dimension = best.unwrap().1;
        sort_nodes(&mut children, dimension, false);
        let mut buckets: Vec<Node> = bucketize(children).into_iter().map(Self::subtree).collect();
        sort_nodes(&mut buckets, dimension, true);
        let buckets = buckets
            .into_iter()
            .map(|bucket| match bucket {
                Self::SubTree { children, .. } => Self::build(children),
                leaf => leaf,
            })
            .collect();
        Self::SubTree {
            space: [(0, 0); 7],
            children: buckets,
        }
        .recompute()
    }

    fn recompute(self) -> Self {
        match self {
            Self::SubTree { children, .. } => Self::subtree(children),
            leaf => leaf,
        }
    }

    #[cfg(test)]
    fn search<'a>(&'a self, target: &[i64; 7], candidate: Option<&'a Node>, bound: i64) -> Option<&'a Node> {
        match self {
            Self::Leaf { .. } => Some(self),
            Self::SubTree { children, .. } => {
                let mut min_distance = candidate.map_or(i64::MAX, |leaf| space_distance(leaf.space(), target));
                let mut closest = candidate;
                for child in children {
                    if let Some(child_distance) = distance_below(child.space(), target, (min_distance - 1).min(bound)) {
                        let Some(leaf) = child.search(target, closest, bound) else { continue };
                        let leaf_distance = if std::ptr::eq(leaf, child) { child_distance } else { space_distance(leaf.space(), target) };
                        if min_distance > leaf_distance {
                            min_distance = leaf_distance;
                            closest = Some(leaf);
                        }
                    }
                }
                closest
            }
        }
    }
}

fn sort_nodes(nodes: &mut [Node], dimension: usize, absolute: bool) {
    let key = |node: &Node, d: usize| {
        let (min, max) = node.space()[d];
        let center = (min + max) / 2;
        if absolute { center.abs() } else { center }
    };
    nodes.sort_by(|a, b| {
        (0..7)
            .map(|offset| (dimension + offset) % 7)
            .map(|d| key(a, d).cmp(&key(b, d)))
            .find(|ordering| ordering.is_ne())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn bucket_size(count: usize) -> usize {
    (CHILDREN_PER_NODE as f64).powf(((count as f64 - 0.01).ln() / (CHILDREN_PER_NODE as f64).ln()).floor()) as usize
}

fn bucket_spaces(nodes: &[Node]) -> Vec<ParameterSpace> {
    nodes
        .chunks(bucket_size(nodes.len()))
        .map(|chunk| {
            let mut space = *chunk[0].space();
            for node in &chunk[1..] {
                for (bound, &(min, max)) in space.iter_mut().zip(node.space()) {
                    *bound = (bound.0.min(min), bound.1.max(max));
                }
            }
            space
        })
        .collect()
}

fn bucketize(nodes: Vec<Node>) -> Vec<Vec<Node>> {
    let size = bucket_size(nodes.len());
    let mut buckets = Vec::new();
    let mut current = Vec::new();
    for node in nodes {
        current.push(node);
        if current.len() >= size {
            buckets.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        buckets.push(current);
    }
    buckets
}

struct DepthGroup {
    depth: (i64, i64),
    tree: Node,
}

pub struct BiomeSource {
    #[cfg(test)]
    root: Node,
    groups: Vec<DepthGroup>,
}

impl BiomeSource {
    pub fn new() -> Self {
        let leaves = OverworldBiomeBuilder::build()
            .into_iter()
            .map(|(point, biome): (ParameterPoint, Biome)| Node::Leaf {
                space: point.map(|parameter| (parameter.min, parameter.max)),
                biome,
                order: 0,
            })
            .collect();
        let mut root = Node::build(leaves);
        root.assign_order(&mut 0);
        let mut ordered = Vec::new();
        root.collect_leaves(&mut ordered);
        let mut by_depth: std::collections::BTreeMap<(i64, i64), Vec<Node>> = std::collections::BTreeMap::new();
        for leaf in ordered {
            by_depth.entry(leaf.space()[DEPTH]).or_default().push(leaf);
        }
        let groups = by_depth.into_iter().map(|(depth, leaves)| DepthGroup { depth, tree: Node::build(leaves) }).collect();
        Self {
            #[cfg(test)]
            root,
            groups,
        }
    }

    pub fn column_hints(&self) -> Vec<Option<Nearest>> {
        vec![None; self.groups.len()]
    }

    pub fn find_column(&self, climate: [f32; 5], depths: &[f32], hints: &mut [Option<Nearest>], out: &mut [Biome]) {
        let q = |v: f32| (v * 10000.0) as i64;
        let target = [q(climate[0]), q(climate[1]), q(climate[2]), q(climate[3]), 0, q(climate[4]), 0];
        let winners: Vec<(Nearest, (i64, i64))> = self
            .groups
            .iter()
            .zip(hints.iter_mut())
            .map(|(group, hint)| {
                let mut best = hint.map(|hint| Nearest {
                    distance: fixed_distance_below(&hint.space, &target, i64::MAX).expect("no limit"),
                    ..hint
                });
                group.tree.nearest_fixed(&target, &mut best);
                *hint = best;
                (best.expect("every depth group has leaves"), group.depth)
            })
            .collect();
        for (&depth, biome) in depths.iter().zip(out) {
            let depth = q(depth);
            let mut best: Option<(i64, usize)> = None;
            for (winner, interval) in &winners {
                let key = (winner.distance + parameter_distance(*interval, depth).pow(2), winner.order);
                if best.is_none_or(|best| key < best) {
                    best = Some(key);
                    *biome = winner.biome;
                }
            }
        }
    }

    #[cfg(test)]
    pub fn find(&self, target: [f32; 6]) -> Biome {
        self.find_near(target, &mut BiomeHint::default())
    }

    #[cfg(test)]
    pub fn find_near<'a>(&'a self, target: [f32; 6], hint: &mut BiomeHint<'a>) -> Biome {
        let q = |v: f32| (v * 10000.0) as i64;
        let target = [q(target[0]), q(target[1]), q(target[2]), q(target[3]), q(target[4]), q(target[5]), 0];
        let bound = hint.0.map_or(i64::MAX, |leaf| space_distance(leaf.space(), &target));
        let leaf = self.root.search(&target, None, bound).expect("the previous leaf is always within the bound");
        hint.0 = Some(leaf);
        let Node::Leaf { biome, .. } = leaf else { unreachable!() };
        *biome
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct BiomeHint<'a>(Option<&'a Node>);
