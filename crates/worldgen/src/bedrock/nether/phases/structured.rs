use super::super::decorate;
use super::super::{CHUNK_AREA, CHUNK_MASK, NetherChunk, NetherGenerator, structure};
use super::terrain::CarverPhase;
use super::{CENTRE, REACH_TWO};
use crate::shared::blocks::BlockId;
use crate::shared::{CHUNK_NEIGHBORHOOD, into_array};
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext};
use glam::IVec2;
use std::sync::Arc;

const START_SIDE: usize = (structure::START_REACH * 2 + 1) as usize;

const PORTAL_SIDE: usize = (structure::portal::REACH * 2 + 1) as usize;

const START_OFFSETS: [(i32, i32); START_SIDE * START_SIDE] = square(structure::START_REACH);

const PORTAL_OFFSETS: [(i32, i32); PORTAL_SIDE * PORTAL_SIDE] = square(structure::portal::REACH);

const fn square<const N: usize>(reach: i32) -> [(i32, i32); N] {
    let mut offsets = [(0, 0); N];
    let mut index = 0;
    let mut x = -reach;
    while x <= reach {
        let mut z = -reach;
        while z <= reach {
            offsets[index] = (x, z);
            index += 1;
            z += 1;
        }
        x += 1;
    }
    offsets
}

fn start_candidate(cell: IVec2) -> bool {
    structure::may_start(cell.x, cell.y)
}

fn portal_candidate(cell: IVec2) -> bool {
    structure::portal::may_start(cell.x, cell.y)
}

pub(in super::super) fn reach_two_index(dx: i32, dz: i32) -> usize {
    ((dx + 2) * 5 + (dz + 2)) as usize
}

pub struct Structured {
    pub chunk: Arc<NetherChunk>,
    pub heights: Arc<[i32; CHUNK_AREA]>,
}

impl NetherGenerator {
    pub(in super::super) fn starts_at(&self, cell: (i32, i32)) -> structure::Starts {
        let quart = |x: i32, z: i32| self.biomes.quart_biome(x, z);
        self.structure.starts_at(cell, &quart)
    }
    pub(in super::super) fn structure_body(&self, cell: (i32, i32), carved: &[Arc<NetherChunk>; 25], starts: &[((i32, i32), Arc<structure::Starts>)], portals: &[(i32, i32)]) -> NetherChunk {
        let mut chunk = (*carved[reach_two_index(0, 0)]).clone();
        let fortresses = self.structure.fortresses_at(cell, starts);
        self.structure.place(&fortresses, cell, &mut chunk, &self.ids, &self.blocks);
        for &start in portals {
            let get = |x: i32, y: i32, z: i32| {
                let (dx, dz) = ((x >> 4) - cell.0, (z >> 4) - cell.1);
                if (dx, dz) == (0, 0) {
                    chunk.get(x & CHUNK_MASK, y, z & CHUNK_MASK)
                } else {
                    carved[reach_two_index(dx, dz)].get(x & CHUNK_MASK, y, z & CHUNK_MASK)
                }
            };
            if let Some(plan) = self.portal_plan(start, &get) {
                plan.apply(&mut chunk, cell);
            }
        }
        let bastions = self.structure.bastions_at(cell, starts);
        if let Some(palette) = self.palette.as_ref() {
            let neighbour = |x: i32, z: i32| carved[reach_two_index(x - cell.0, z - cell.1)].clone();
            self.structure.place_bastions(&bastions, cell, &mut chunk, &self.ids, &self.blocks, palette, &neighbour);
        }
        chunk
    }

    fn portal_plan(&self, start: (i32, i32), get: &dyn Fn(i32, i32, i32) -> BlockId) -> Option<structure::portal::PortalPlan> {
        let (data, palette) = (self.data.as_ref()?, self.palette.as_ref()?);
        let library = data.library.as_ref()?;
        Some(structure::portal::plan(start, self.seed, self.definition, get, &self.ids, &self.blocks, palette, library))
    }
    pub(in super::super) fn finish_structure(&self, cell: (i32, i32), mut body: NetherChunk, bodies: &[Arc<NetherChunk>; 9]) -> Structured {
        let neighbour = |x: i32, z: i32| bodies[((x - cell.0 + 1) * 3 + (z - cell.1 + 1)) as usize].clone();
        self.structure.join(&mut body, cell, &neighbour, &self.ids, &self.blocks);
        let heights = decorate::light_heights(&self.blocks, &body);
        Structured {
            chunk: Arc::new(body),
            heights: Arc::new(heights),
        }
    }
}

pub struct StartsPhase;

impl Phase<NetherGenerator> for StartsPhase {
    type Output = structure::Starts;

    const RETAIN: usize = 1024;

    fn run(generator: &NetherGenerator, cell: IVec2, _inputs: &mut PhaseContext<NetherGenerator>) -> Result<structure::Starts, PhaseError> {
        Ok(generator.starts_at((cell.x, cell.y)))
    }
}

pub struct PortalStartPhase;

impl Phase<NetherGenerator> for PortalStartPhase {
    type Output = bool;

    const RETAIN: usize = 1024;

    fn run(generator: &NetherGenerator, cell: IVec2, _inputs: &mut PhaseContext<NetherGenerator>) -> Result<bool, PhaseError> {
        Ok(structure::portal::is_start(generator.seed, (cell.x, cell.y)))
    }
}

pub struct StructureBodyPhase;

impl Phase<NetherGenerator> for StructureBodyPhase {
    type Output = Arc<NetherChunk>;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![
            Dependency::on::<CarverPhase>(REACH_TWO),
            Dependency::on::<StartsPhase>(&START_OFFSETS).if_dependency_position(start_candidate),
            Dependency::on::<PortalStartPhase>(&PORTAL_OFFSETS).if_dependency_position(portal_candidate),
        ]
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<Arc<NetherChunk>, PhaseError> {
        let mut carved: Vec<Arc<NetherChunk>> = Vec::with_capacity(REACH_TWO.len());
        for &(dx, dz) in REACH_TWO {
            carved.push(context.get::<CarverPhase>(cell + IVec2::new(dx, dz))?);
        }
        let carved: [Arc<NetherChunk>; 25] = into_array(carved);
        let mut starts: Vec<((i32, i32), Arc<structure::Starts>)> = Vec::new();
        for &(dx, dz) in &START_OFFSETS {
            let at = cell + IVec2::new(dx, dz);
            if start_candidate(at) {
                starts.push(((at.x, at.y), context.get::<StartsPhase>(at)?));
            }
        }
        let mut portals: Vec<(i32, i32)> = Vec::new();
        for &(dx, dz) in &PORTAL_OFFSETS {
            let at = cell + IVec2::new(dx, dz);
            if portal_candidate(at) && *context.get::<PortalStartPhase>(at)? {
                portals.push((at.x, at.y));
            }
        }
        let mut body = generator.structure_body((cell.x, cell.y), &carved, &starts, &portals);
        body.compact();
        Ok(Arc::new(body))
    }
}

pub struct StructureJoinPhase;

impl Phase<NetherGenerator> for StructureJoinPhase {
    type Output = Arc<Structured>;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<StructureBodyPhase>(CHUNK_NEIGHBORHOOD)]
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<Arc<Structured>, PhaseError> {
        let mut bodies: Vec<Arc<NetherChunk>> = Vec::with_capacity(CHUNK_NEIGHBORHOOD.len());
        for &(dx, dz) in CHUNK_NEIGHBORHOOD {
            bodies.push((*context.get::<StructureBodyPhase>(cell + IVec2::new(dx, dz))?).clone());
        }
        let bodies: [Arc<NetherChunk>; 9] = into_array(bodies);
        let body = (*bodies[CENTRE]).clone();
        let mut structured = generator.finish_structure((cell.x, cell.y), body, &bodies);
        Arc::make_mut(&mut structured.chunk).compact();
        Ok(Arc::new(structured))
    }
}
