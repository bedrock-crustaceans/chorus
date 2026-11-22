use std::sync::Arc;

use rustc_hash::FxHashMap;
use tracing::warn;

use super::EndGenerator;
use super::cities::{self, City};
use super::phases::StructureStartPhase;
use crate::bedrock::demand::Demand;
use chorus_level::generator::engine::{Delivery, LocateJob, LocateStep, Located};
use chorus_level::generator::error::PhaseFailure;
use chorus_level::generator::phase::{PhaseId, Request};

use crate::shared::biome::pack_ids::THE_END;
use crate::shared::structure::pack_biome_id;
use glam::IVec2;

const CITY_NAME: &str = "end_city";
const RINGS: i32 = 100;
const BATCH: usize = 8;

pub(super) fn job(generator: Arc<EndGenerator>, name: &str, from: (i32, i32), _radius: i32) -> Option<Box<dyn LocateJob<EndGenerator>>> {
    if let Some(requested) = name.strip_prefix("biome:") {
        if pack_biome_id(requested.strip_prefix("minecraft:").unwrap_or(requested))? != THE_END {
            return None;
        }
        let y = crate::shared::SEA_LEVEL;
        return Some(Box::new(chorus_level::generator::engine::Immediate(Some(Located {
            x: from.0.div_euclid(16),
            z: from.1.div_euclid(16),
            y: Some(y),
            position: Some(from),
        }))));
    }
    if name != CITY_NAME {
        return None;
    }
    Some(Box::new(Search {
        generator,
        from,
        starts: FxHashMap::default(),
        demand: Demand::new(),
    }))
}

struct Search {
    generator: Arc<EndGenerator>,
    from: (i32, i32),
    starts: FxHashMap<(i32, i32), bool>,
    demand: Demand,
}

impl Search {
    fn absorb(&mut self, arrivals: Vec<Delivery>) -> Result<(), Arc<PhaseFailure>> {
        for delivery in arrivals {
            let output = delivery.outcome?;
            if delivery.phase == PhaseId::of::<StructureStartPhase>()
                && let Ok(city) = output.downcast::<Option<Arc<City>>>()
            {
                self.starts.insert((delivery.cell.x, delivery.cell.y), city.is_some());
            }
        }
        Ok(())
    }

    fn demand(&mut self, misses: Vec<(i32, i32)>) -> LocateStep<EndGenerator> {
        self.demand.step(misses.into_iter().map(|start| Request::of::<StructureStartPhase>(IVec2::new(start.0, start.1))))
    }
}

impl LocateJob<EndGenerator> for Search {
    fn step(&mut self, arrived: Vec<Delivery>) -> LocateStep<EndGenerator> {
        if let Err(failure) = self.absorb(arrived) {
            warn!("a structure search failed: {failure}");
            return LocateStep::Done(None);
        }
        let spacing = cities::GRID.spacing;
        let centre = (self.from.0.div_euclid(spacing), self.from.1.div_euclid(spacing));
        let mut misses: Vec<(i32, i32)> = Vec::new();
        for ring in 0..=RINGS {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let start = cities::GRID.candidate(self.generator.seed(), ((centre.0 + dx) * spacing, (centre.1 + dz) * spacing));
                    if !cities::beyond_origin(start) {
                        continue;
                    }
                    match self.starts.get(&start) {
                        Some(true) if misses.is_empty() => {
                            return LocateStep::Done(Some(Located {
                                x: start.0,
                                z: start.1,
                                y: None,
                                position: None,
                            }));
                        }
                        Some(true) => return self.demand(misses),
                        Some(false) => {}
                        None => misses.push(start),
                    }
                }
            }
            if misses.len() >= BATCH {
                break;
            }
        }
        if misses.is_empty() { LocateStep::Done(None) } else { self.demand(misses) }
    }
}
