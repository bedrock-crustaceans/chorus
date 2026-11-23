mod aquatic;
mod cave;
mod coral;
mod disk;
pub mod expression;
#[cfg(test)]
mod feature_tests;
pub(super) mod filter;
mod fossil;
mod geode;
mod ice;
mod iceberg;
mod jungle;
mod lake;
mod land;
pub mod ore;
mod pale;
mod pile;
pub mod placed;
mod plants;
mod pool;
pub mod registry;
pub mod rules;
pub mod scatter;
pub(super) mod seed;
use crate::shared::math::sine;
mod single_block;
mod springs;
mod structure_template;
#[cfg(test)]
mod tests;
mod tree;
mod vegetation;
mod well;
mod world;

use crate::shared::biome::Biome;
use crate::shared::blocks::BlockTable;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use expression::Variables;
use glam::IVec3;
use lake::{LavaLake, MonsterRoom};
use placed::Placed;
use registry::FeatureRegistry;
use rules::{Feature, Rule};
use scatter::Draws;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use world::biome_id_at;

const SECTION_SIZE: i32 = 16;

pub struct Decoration {
    level_seed: u32,
    rules: BTreeMap<(usize, &'static str), Rule>,
    lava_lake: LavaLake,
    room: MonsterRoom,
    features: HashMap<Arc<str>, Arc<Placed>>,
}

impl Rule {
    pub(super) fn runs(&self) -> bool {
        self.scatter.as_ref().is_some_and(|scatter| !scatter.is_inert()) && !self.biomes.is_empty()
    }
}

fn biome_order(region: &Region, _owner_x: i32, _owner_z: i32) -> Vec<i32> {
    let mut order: Vec<i32> = Vec::new();
    let mut section = region.min_y();
    while section <= region.max_y() {
        if let Some(biome) = region.uniform_biome_id_at(section) {
            if !order.contains(&biome) {
                order.push(biome);
            }
            section += SECTION_SIZE;
            continue;
        }
        let (palette_len, ids) = region.owner_biome_section(section);
        let mut found: Vec<i32> = Vec::with_capacity(palette_len);
        'scan: for x in 0..SECTION_SIZE as usize {
            for z in 0..SECTION_SIZE as usize {
                for y in 0..SECTION_SIZE as usize {
                    let biome = ids[(y << 8) | (z << 4) | x];
                    if !found.contains(&biome) {
                        found.push(biome);
                        if !order.contains(&biome) {
                            order.push(biome);
                        }
                        if found.len() == palette_len {
                            break 'scan;
                        }
                    }
                }
            }
        }
        section += SECTION_SIZE;
    }
    order
}

impl Decoration {
    pub fn new(level_seed: u32, table: &mut BlockTable, extra_features: &[Arc<str>], registry: &FeatureRegistry) -> Self {
        let loaded = rules::load_with_registry(table, extra_features, registry);
        let rules: BTreeMap<(usize, &'static str), Rule> = loaded.rules.into_iter().filter(|(_, rule)| rule.runs()).collect();
        tracing::debug!("bedrock decoration runs {} of {} feature rules", rules.len(), loaded.report.rule_count);
        Self {
            level_seed,
            rules,
            lava_lake: LavaLake::new(table),
            room: MonsterRoom::new(table),
            features: loaded.features,
        }
    }

    pub fn knows(&self, id: &str) -> bool {
        self.features.contains_key(id)
    }

    pub fn place_named(&self, region: &mut Region, id: &str, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        match self.features.get(id) {
            Some(feature) => Ok(feature.place_at(region, random, pos)?.is_some()),
            None => Ok(false),
        }
    }

    fn lakes_and_rooms(&self, region: &mut Region, random: &mut MersenneTwister, owner_x: i32, owner_z: i32) {
        if let Some(origin) = lake::draw_lava_origin(random, owner_x, owner_z, region.sea_level()) {
            self.lava_lake.place(region, random, origin);
        }
        let min_y = region.min_y();
        lake::room_origins(random, owner_x, owner_z, min_y, |random, origin| {
            self.room.place(region, random, origin);
        });
    }

    pub fn decorate(&self, region: &mut Region, owner_x: i32, owner_z: i32) -> Probed<()> {
        let decoration_seed = seed::decoration_seed(self.level_seed, owner_x, owner_z);
        self.lakes_and_rooms(region, &mut MersenneTwister::new(decoration_seed), owner_x, owner_z);
        let order = biome_order(region, owner_x, owner_z);
        let corner = IVec3::new(owner_x << 4, 0, owner_z << 4);
        for pass in 0..rules::PASSES.len() {
            for &id in &order {
                let biome = Biome::generated(id);
                scatter::reset_origin();
                let only = (order.len() > 1).then_some(id);
                for (_, rule) in self.rules.range((pass, "")..(pass + 1, "")) {
                    if rule.biomes.contains(&biome) {
                        self.run_rule(rule, region, decoration_seed, corner, only)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn run_rule(&self, rule: &Rule, region: &mut Region, decoration_seed: u32, corner: IVec3, only: Option<i32>) -> Probed<()> {
        scatter::reset_world();
        let Some(scatter) = &rule.scatter else { return Ok(()) };
        let placed = match &rule.feature {
            Feature::Placed(placed) => Some(placed),
            Feature::Unsupported(_) if scatter.uses_random() => None,
            Feature::Unsupported(_) => return Ok(()),
        };
        let rule_seed = seed::rule_seed(rule.hash, decoration_seed);
        let mut positions = MersenneTwister::new(rule_seed);
        let mut molang = MersenneTwister::new(rule_seed);
        let mut placing: Option<MersenneTwister> = None;
        let variables = Variables {
            origin: scatter::current_origin(corner),
            world: [0.0; 3],
        };
        let mut draws = Draws {
            positions: &mut positions,
            molang: Some(&mut molang),
        };
        scatter.try_run(&mut draws, variables, corner, region, |region, _, pos| {
            let Some(placed) = placed else { return Ok(()) };
            if only.is_some_and(|wanted| biome_id_at(region, pos) != wanted) {
                return Ok(());
            }
            placed.place(region, placing.get_or_insert_with(|| MersenneTwister::new(rule_seed)), pos)?;
            Ok(())
        })
    }
}
