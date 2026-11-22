use std::sync::Arc;

use rustc_hash::FxHashMap;
use tracing::warn;

use super::NetherGenerator;
use super::phases::{PortalStartPhase, StartsPhase};
use super::structure::{NETHER_GRID, SharedKind, Starts, portal};
use crate::bedrock::biome_locate::BiomeSearch;
use crate::bedrock::demand::Demand;
use crate::shared::SEA_LEVEL;
use crate::shared::structure::pack_biome_id;
use chorus_level::generator::engine::{Delivery, LocateJob, LocateStep, Located};
use chorus_level::generator::phase::{PhaseHandle, PhaseId, Request};
use glam::IVec2;

const RINGS: i32 = 100;
const BATCH: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Fortress,
    Bastion,
    Portal,
}

pub(super) fn job(generator: Arc<NetherGenerator>, name: &str, from: (i32, i32), radius: i32) -> Option<Box<dyn LocateJob<NetherGenerator>>> {
    if let Some(requested) = name.strip_prefix("biome:") {
        let target = pack_biome_id(requested.strip_prefix("minecraft:").unwrap_or(requested))?;
        let source = generator.clone();
        let sample = Box::new(move |x: i32, z: i32| (source.biome_at(x, z) == target).then_some(SEA_LEVEL));
        return Some(Box::new(BiomeSearch::new(from, radius, sample)));
    }
    let kind = match name {
        "fortress" => Kind::Fortress,
        "bastion" => Kind::Bastion,
        "ruined_portal" => Kind::Portal,
        _ => return None,
    };
    Some(Box::new(Search {
        generator,
        kind,
        from,
        starts: FxHashMap::default(),
        portals: FxHashMap::default(),
        demand: Demand::new(),
    }))
}

struct Search {
    generator: Arc<NetherGenerator>,
    kind: Kind,
    from: (i32, i32),
    starts: FxHashMap<(i32, i32), Arc<Starts>>,
    portals: FxHashMap<(i32, i32), bool>,
    demand: Demand,
}

impl Search {
    fn absorb(&mut self, arrivals: Vec<Delivery>) -> Result<(), Arc<chorus_level::generator::error::PhaseFailure>> {
        for delivery in arrivals {
            let cell = (delivery.cell.x, delivery.cell.y);
            let output = delivery.outcome?;
            if delivery.phase == PhaseId::of::<StartsPhase>() {
                if let Ok(starts) = output.downcast::<Starts>() {
                    self.starts.insert(cell, starts);
                }
            } else if delivery.phase == PhaseId::of::<PortalStartPhase>()
                && let Ok(portal) = output.downcast::<bool>()
            {
                self.portals.insert(cell, *portal);
            }
        }
        Ok(())
    }

    fn verdict(&self, chunk: (i32, i32)) -> Option<bool> {
        match self.kind {
            Kind::Portal => self.portals.get(&chunk).copied(),
            Kind::Fortress => self.starts.get(&chunk).map(|starts| starts.kind == Some(SharedKind::Fortress)),
            Kind::Bastion => self.starts.get(&chunk).map(|starts| starts.kind == Some(SharedKind::Bastion)),
        }
    }

    fn handle(&self) -> PhaseHandle<NetherGenerator> {
        match self.kind {
            Kind::Portal => PhaseHandle::of::<PortalStartPhase>(),
            _ => PhaseHandle::of::<StartsPhase>(),
        }
    }

    fn demand(&mut self, misses: Vec<(i32, i32)>) -> LocateStep<NetherGenerator> {
        let handle = self.handle();
        self.demand.step(misses.into_iter().map(|chunk| Request {
            handle,
            cell: IVec2::new(chunk.0, chunk.1),
        }))
    }

    fn found(&mut self, chunk: (i32, i32), misses: Vec<(i32, i32)>) -> LocateStep<NetherGenerator> {
        if misses.is_empty() {
            LocateStep::Done(Some(Located {
                x: chunk.0,
                z: chunk.1,
                y: None,
                position: None,
            }))
        } else {
            self.demand(misses)
        }
    }
}

impl LocateJob<NetherGenerator> for Search {
    fn step(&mut self, arrived: Vec<Delivery>) -> LocateStep<NetherGenerator> {
        if let Err(failure) = self.absorb(arrived) {
            warn!("a structure search failed: {failure}");
            return LocateStep::Done(None);
        }
        let grid = if self.kind == Kind::Portal { &portal::PORTAL_GRID } else { &NETHER_GRID };
        let seed = self.generator.seed;
        let mut starts: FxHashMap<(i32, i32), (i32, i32)> = FxHashMap::default();
        let mut misses: Vec<(i32, i32)> = Vec::new();
        for ring in 0..=RINGS {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let chunk = (self.from.0 + dx, self.from.1 + dz);
                    let start = *starts.entry(grid.region(chunk)).or_insert_with(|| grid.candidate(seed, chunk));
                    if start != chunk {
                        continue;
                    }
                    match self.verdict(chunk) {
                        Some(true) => return self.found(chunk, misses),
                        Some(false) => {}
                        None => misses.push(chunk),
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
