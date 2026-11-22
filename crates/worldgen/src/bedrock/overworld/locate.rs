use std::sync::Arc;

use tracing::warn;

use super::OverworldGenerator;
use super::phases::{CarverPhase, HeightsPhase, StartsPhase};
use super::structure_probe::{Arrived, BedrockProbe, Supply};
use crate::bedrock::biome_locate::BiomeSearch;
use crate::bedrock::demand::Demand;
use crate::shared::biome::Biome;
use crate::shared::structure::{Probed, Remembering, StructureKind};
use chorus_level::generator::engine::{Delivery, LocateJob, LocateStep, Located};
use chorus_level::generator::phase::Request;
use glam::IVec2;

const SPAWN: &str = "spawn";
const CHUNK_MASK: i32 = 15;
const BIOME_PREFIX: &str = "biome:";

pub(super) fn job(generator: Arc<OverworldGenerator>, name: &str, from: (i32, i32), radius: i32) -> Option<Box<dyn LocateJob<OverworldGenerator>>> {
    if let Some(requested) = name.strip_prefix(BIOME_PREFIX) {
        let named = requested.strip_prefix("minecraft:").unwrap_or(requested);
        let target = Biome::ALL.into_iter().find(|biome| biome.pack_name() == named)?;
        let source = generator.clone();
        let sample = Box::new(move |x: i32, z: i32| {
            let terrain = &source.terrain;
            let surface = terrain.preliminary_surface(x >> 2, z >> 2).max(terrain.sea_level());
            let mut probe = surface;
            loop {
                if source.biomes.point(terrain, x, probe, z) == target {
                    let mut y = probe;
                    while y < surface && source.biomes.point(terrain, x, y + 1, z) == target {
                        y += 1;
                    }
                    return Some(y);
                }
                if probe <= terrain.min_y() {
                    return None;
                }
                probe = (probe - 64).max(terrain.min_y());
            }
        });
        return Some(Box::new(BiomeSearch::new(from, radius, sample)));
    }
    if name == SPAWN {
        let column = generator.spawn_column();
        return Some(Box::new(Spawn {
            generator,
            column,
            supplied: Arrived::default(),
        }));
    }
    let kind = generator.structures()?.registry().kind_named(name)?;
    Some(Box::new(Search {
        generator,
        kind,
        from,
        supplied: Arrived::default(),
        demand: Demand::new(),
    }))
}

struct Search {
    generator: Arc<OverworldGenerator>,
    kind: StructureKind,
    from: (i32, i32),
    supplied: Arrived,
    demand: Demand,
}

impl LocateJob<OverworldGenerator> for Search {
    fn step(&mut self, arrived: Vec<Delivery>) -> LocateStep<OverworldGenerator> {
        if let Err(failure) = self.supplied.absorb(arrived) {
            warn!("a structure search failed: {failure}");
            return LocateStep::Done(None);
        }
        let structures = self.generator.structures().expect("a search is only made for a generator with structures");
        let base = BedrockProbe::reading(&self.generator, &self.supplied);
        let probe = Remembering::dry(&base);
        let found: Probed<Option<(i32, i32)>> = if self.kind == StructureKind::Stronghold {
            structures.locate_stronghold(&probe, self.from)
        } else {
            structures.locate(&probe, self.kind, self.from)
        };
        let found = match found {
            Ok(found) => found,
            Err(missing) => {
                warn!("a structure search could not read its inputs: {missing}");
                return LocateStep::Done(None);
            }
        };
        let mut demands: Vec<Request<OverworldGenerator>> = Vec::new();
        demands.extend(probe.missed_heights().into_iter().map(|(x, z)| Request::of::<HeightsPhase>(IVec2::new(x, z))));
        demands.extend(probe.missed_starts().into_iter().map(|(x, z)| Request::of::<StartsPhase>(IVec2::new(x, z))));
        demands.extend(probe.missed_chunks().into_iter().map(|(x, z)| Request::of::<CarverPhase>(IVec2::new(x, z))));
        let demands = self.demand.fresh(demands);
        if demands.is_empty() {
            LocateStep::Done(found.map(|(x, z)| Located { x, z, y: None, position: None }))
        } else {
            LocateStep::Demand(demands)
        }
    }
}

struct Spawn {
    generator: Arc<OverworldGenerator>,
    column: (i32, i32),
    supplied: Arrived,
}

impl LocateJob<OverworldGenerator> for Spawn {
    fn step(&mut self, arrived: Vec<Delivery>) -> LocateStep<OverworldGenerator> {
        if let Err(failure) = self.supplied.absorb(arrived) {
            warn!("the spawn search failed: {failure}");
            return LocateStep::Done(None);
        }
        let (x, z) = self.column;
        let chunk_pos = (x >> 4, z >> 4);
        let Some(chunk) = self.supplied.chunk(chunk_pos) else {
            return LocateStep::Demand(vec![Request::of::<CarverPhase>(IVec2::new(chunk_pos.0, chunk_pos.1))]);
        };
        let y = chunk.surface_height(x & CHUNK_MASK, z & CHUNK_MASK);
        LocateStep::Done(Some(Located {
            x,
            z,
            y: Some(y.max(self.generator.terrain.min_y()) + 1),
            position: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use chorus_block::block_registry::BlockRegistry;
    use chorus_level::dimension_type::DimensionType;
    use chorus_level::generator::dimension::Dimension;

    const GENERAL_1: i64 = 4802433367836389877;

    fn registry() -> BlockRegistry {
        let mut registry = BlockRegistry::new();
        registry.register_all(chorus_block::r#impl::DEFINITIONS.iter().copied());
        registry
    }

    #[test]
    fn the_spawn_job_finds_dry_land_through_the_graph() {
        bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
        let mut dimension = Dimension::new(DimensionType::Overworld, super::super::OverworldGenerator::new(GENERAL_1, &registry()).expect("the generator builds"));
        let job = dimension.start_locate("spawn", (0, 0), 0).expect("a spawn job");
        let located = loop {
            dimension.tick();
            if let Some((_, found)) = dimension.take_located().into_iter().find(|(id, _)| *id == job) {
                break found.expect("a spawn");
            }
            std::thread::yield_now();
        };
        assert!(located.y.expect("a spawn height") > crate::shared::SEA_LEVEL);
    }

    #[test]
    fn a_biome_search_matches_the_coarse_grid() {
        use crate::shared::SEA_LEVEL;
        use crate::shared::biome::Biome;
        bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
        let target = Biome::OCEAN;
        let source = super::super::OverworldGenerator::new(GENERAL_1, &registry()).expect("the generator builds");
        let matches = |x: i32, z: i32| {
            let surface = source.terrain.preliminary_surface(x >> 2, z >> 2).max(SEA_LEVEL);
            let mut probe = surface;
            loop {
                if source.biomes.point(&source.terrain, x, probe, z) == target {
                    let mut y = probe;
                    while y < surface && source.biomes.point(&source.terrain, x, y + 1, z) == target {
                        y += 1;
                    }
                    return Some(y);
                }
                if probe <= chorus_level::dimension_definition::DimensionDefinition::OVERWORLD.min_y {
                    return None;
                }
                probe = (probe - 64).max(chorus_level::dimension_definition::DimensionDefinition::OVERWORLD.min_y);
            }
        };
        let mut first_ring = i32::MAX;
        for z in -20..=20 {
            for x in -20..=20 {
                if x == 0 && z == 0 {
                    continue;
                }
                if matches(x * 32, z * 32).is_some() {
                    first_ring = first_ring.min(x.abs().max(z.abs()));
                }
            }
        }
        let mut dimension = Dimension::new(DimensionType::Overworld, super::super::OverworldGenerator::new(GENERAL_1, &registry()).expect("the generator builds"));
        let job = dimension.start_locate("biome:ocean", (0, 0), 640).expect("a biome job");
        let located = loop {
            dimension.tick();
            if let Some((_, found)) = dimension.take_located().into_iter().find(|(id, _)| *id == job) {
                break found.expect("an ocean");
            }
            std::thread::yield_now();
        };
        let (x, z) = located.position.expect("a position");
        assert_eq!(Some(located.y.expect("a height")), matches(x, z));
        assert_eq!((x / 32).abs().max((z / 32).abs()), first_ring);
    }
}
