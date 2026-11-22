use crate::bedrock::overworld::decoration::placed::Placed;
use crate::bedrock::overworld::decoration::scatter::{bind_world, leave_origin};
use crate::bedrock::overworld::decoration::world::biome_at;
use crate::bedrock::overworld::surface::data;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use glam::IVec3;
use std::sync::Arc;

pub const FEATURE: &str = "minecraft:optional_beehive_feature";
const FULL_PERCENT: f32 = 100.0;
const HABITAT: &str = "bee_habitat";
const PLAINS_PERCENT: f32 = 5.0;
const FLOWER_FOREST_PERCENT: f32 = 3.0;
const FOREST_PERCENT: f32 = 0.035;
const PLAINS_TAGS: [&str; 2] = ["plains", "sunflower_plains"];
const FLOWER_FOREST_TAG: &str = "flower_forest";
const FOREST_TAGS: [&str; 5] = ["birch_forest", "birch_forest_hills", "birch_forest_mutated", "forest", "forest_mutated"];

#[derive(Debug)]
pub struct OptionalBeehive {
    pub search: Arc<Placed>,
}

fn chance(tags: &[&str]) -> f32 {
    let has = |wanted: &str| tags.contains(&wanted);
    if !has(HABITAT) {
        0.0
    } else if PLAINS_TAGS.iter().any(|tag| has(tag)) {
        PLAINS_PERCENT
    } else if has(FLOWER_FOREST_TAG) {
        FLOWER_FOREST_PERCENT
    } else if FOREST_TAGS.iter().any(|tag| has(tag)) {
        FOREST_PERCENT
    } else {
        0.0
    }
}

impl OptionalBeehive {
    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<Option<IVec3>> {
        let bound = bind_world();
        let percent = if bound {
            let tags: Vec<&str> = data::tags(&biome_at(region, pos)).collect();
            chance(&tags)
        } else {
            0.0
        };
        let passes = if percent >= FULL_PERCENT {
            true
        } else if percent <= 0.0 {
            false
        } else {
            random.next_float() * FULL_PERCENT < percent
        };
        let placed = if passes { self.search.place_at(region, random, pos)? } else { None };
        leave_origin(pos);
        Ok(placed)
    }
}
