use super::super::decorate;
use super::super::{CHUNK_AREA, CHUNK_MASK, NetherChunk, NetherGenerator, structure};
use super::terrain::CarverPhase;
use super::{CENTRE, REACH_TWO};
use crate::shared::blocks::BlockId;
use crate::shared::decoration_window::offset_slot;
use crate::shared::{CHUNK_NEIGHBORHOOD, into_array};
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext, Request, SAME_CELL};
use glam::IVec2;
use std::cell::Cell;
use std::sync::Arc;

pub(in super::super) fn reach_two_index(dx: i32, dz: i32) -> Option<usize> {
    ((-2..=2).contains(&dx) && (-2..=2).contains(&dz)).then(|| ((dx + 2) * 5 + (dz + 2)) as usize)
}

fn undeclared(dependency: &'static str, missed: &Cell<Option<(i32, i32)>>) -> Result<(), PhaseError> {
    match missed.get() {
        Some((x, z)) => Err(PhaseError::UndeclaredRead { dependency, cell: IVec2::new(x, z) }),
        None => Ok(()),
    }
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
    pub(in super::super) fn structure_body(
        &self,
        cell: (i32, i32),
        carved: &[Option<Arc<NetherChunk>>; 25],
        starts: &[((i32, i32), Arc<structure::Starts>)],
        portals: &[(i32, i32)],
    ) -> Result<NetherChunk, PhaseError> {
        let missed = Cell::new(None);
        let Some(centre) = carved[reach_two_index(0, 0).unwrap_or_default()].as_ref() else {
            return Err(PhaseError::UndeclaredRead {
                dependency: "CarverPhase",
                cell: IVec2::new(cell.0, cell.1),
            });
        };
        let carved_at = |dx: i32, dz: i32| match reach_two_index(dx, dz).and_then(|index| carved[index].as_ref()) {
            Some(chunk) => chunk,
            None => {
                missed.set(Some((cell.0 + dx, cell.1 + dz)));
                centre
            }
        };
        let mut chunk = (**centre).clone();
        let fortresses = self.structure.fortresses_at(cell, starts);
        self.structure.place(&fortresses, cell, &mut chunk, &self.ids, &self.blocks);
        for &start in portals {
            let get = |x: i32, y: i32, z: i32| {
                let (dx, dz) = ((x >> 4) - cell.0, (z >> 4) - cell.1);
                if (dx, dz) == (0, 0) {
                    chunk.get(x & CHUNK_MASK, y, z & CHUNK_MASK)
                } else {
                    carved_at(dx, dz).get(x & CHUNK_MASK, y, z & CHUNK_MASK)
                }
            };
            if let Some(plan) = self.portal_plan(start, &get) {
                plan.apply(&mut chunk, cell);
            }
        }
        let bastions = self.structure.bastions_at(cell, starts);
        if let Some(palette) = self.palette.as_ref() {
            let neighbour = |x: i32, z: i32| carved_at(x - cell.0, z - cell.1).clone();
            self.structure.place_bastions(&bastions, cell, &mut chunk, &self.ids, &self.blocks, palette, &neighbour);
        }
        undeclared("CarverPhase", &missed)?;
        Ok(chunk)
    }

    fn portal_plan(&self, start: (i32, i32), get: &dyn Fn(i32, i32, i32) -> BlockId) -> Option<structure::portal::PortalPlan> {
        let (data, palette) = (self.data.as_ref()?, self.palette.as_ref()?);
        let library = data.library.as_ref()?;
        Some(structure::portal::plan(start, self.seed, self.definition, get, &self.ids, &self.blocks, palette, library))
    }
    pub(in super::super) fn finish_structure(&self, cell: (i32, i32), mut body: NetherChunk, bodies: &[Option<Arc<NetherChunk>>; 9]) -> Result<Structured, PhaseError> {
        let missed = Cell::new(None);
        let within = |x: i32, z: i32| (x - cell.0).abs() <= 1 && (z - cell.1).abs() <= 1;
        let neighbour = |x: i32, z: i32| match bodies.get(offset_slot(cell, (x, z))).and_then(Option::as_ref).filter(|_| within(x, z)) {
            Some(chunk) => chunk.clone(),
            None => {
                missed.set(Some((x, z)));
                Arc::new(NetherChunk::empty(self.definition))
            }
        };
        self.structure.join(&mut body, cell, &neighbour, &self.ids, &self.blocks);
        undeclared("StructureBodyPhase", &missed)?;
        let heights = decorate::light_heights(&self.blocks, &body);
        Ok(Structured {
            chunk: Arc::new(body),
            heights: Arc::new(heights),
        })
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
        vec![Dependency::on::<CarverPhase>(SAME_CELL)]
    }

    fn resolve(generator: &NetherGenerator, cell: IVec2, context: &PhaseContext<NetherGenerator>) -> Vec<Request<NetherGenerator>> {
        let start_cells = generator.structure.start_cells_near((cell.x, cell.y));
        let portal_cells = structure::portal::start_cells_near(generator.seed, (cell.x, cell.y));
        let mut requests: Vec<Request<NetherGenerator>> = Vec::new();
        requests.extend(start_cells.iter().map(|&(x, z)| Request::of::<StartsPhase>(IVec2::new(x, z))));
        requests.extend(portal_cells.iter().map(|&(x, z)| Request::of::<PortalStartPhase>(IVec2::new(x, z))));
        let starts: Option<Vec<((i32, i32), Arc<structure::Starts>)>> = start_cells.iter().map(|&(x, z)| Some(((x, z), context.try_get::<StartsPhase>(IVec2::new(x, z))?))).collect();
        if let Some(starts) = starts
            && (!portal_cells.is_empty() || !generator.structure.bastions_at((cell.x, cell.y), &starts).is_empty())
        {
            requests.extend(
                REACH_TWO
                    .iter()
                    .filter(|&&offset| offset != (0, 0))
                    .map(|&(dx, dz)| Request::of::<CarverPhase>(cell + IVec2::new(dx, dz))),
            );
        }
        requests
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<Arc<NetherChunk>, PhaseError> {
        let mut carved: Vec<Option<Arc<NetherChunk>>> = Vec::with_capacity(REACH_TWO.len());
        for &(dx, dz) in REACH_TWO {
            carved.push(context.try_get::<CarverPhase>(cell + IVec2::new(dx, dz)));
        }
        let carved: [Option<Arc<NetherChunk>>; 25] = into_array(carved);
        let mut starts: Vec<((i32, i32), Arc<structure::Starts>)> = Vec::new();
        for (x, z) in generator.structure.start_cells_near((cell.x, cell.y)) {
            starts.push(((x, z), context.get::<StartsPhase>(IVec2::new(x, z))?));
        }
        let mut portals: Vec<(i32, i32)> = Vec::new();
        for (x, z) in structure::portal::start_cells_near(generator.seed, (cell.x, cell.y)) {
            if *context.get::<PortalStartPhase>(IVec2::new(x, z))? {
                portals.push((x, z));
            }
        }
        let mut body = generator.structure_body((cell.x, cell.y), &carved, &starts, &portals)?;
        body.compact();
        Ok(Arc::new(body))
    }
}

pub struct StructureJoinPhase;

impl Phase<NetherGenerator> for StructureJoinPhase {
    type Output = Arc<Structured>;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<StructureBodyPhase>(SAME_CELL)]
    }

    fn resolve(generator: &NetherGenerator, cell: IVec2, context: &PhaseContext<NetherGenerator>) -> Vec<Request<NetherGenerator>> {
        let Some(body) = context.try_get::<StructureBodyPhase>(cell) else { return Vec::new() };
        if !generator.structure.joins_reach_neighbours(&body, &generator.ids, &generator.blocks) {
            return Vec::new();
        }
        CHUNK_NEIGHBORHOOD
            .iter()
            .filter(|&&offset| offset != (0, 0))
            .map(|&(dx, dz)| Request::of::<StructureBodyPhase>(cell + IVec2::new(dx, dz)))
            .collect()
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<Arc<Structured>, PhaseError> {
        let mut bodies: Vec<Option<Arc<NetherChunk>>> = Vec::with_capacity(CHUNK_NEIGHBORHOOD.len());
        for &(dx, dz) in CHUNK_NEIGHBORHOOD {
            bodies.push(context.try_get::<StructureBodyPhase>(cell + IVec2::new(dx, dz)).map(|body| (*body).clone()));
        }
        let bodies: [Option<Arc<NetherChunk>>; 9] = into_array(bodies);
        let body = (**bodies[CENTRE].as_ref().expect("the join's own body is a declared dependency")).clone();
        let mut structured = generator.finish_structure((cell.x, cell.y), body, &bodies)?;
        Arc::make_mut(&mut structured.chunk).compact();
        Ok(Arc::new(structured))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chorus_block::block_registry::BlockRegistry;

    fn generator() -> NetherGenerator {
        let mut registry = BlockRegistry::new();
        registry.register_all(chorus_block::r#impl::DEFINITIONS.iter().copied());
        NetherGenerator::new(1, &registry)
    }

    fn body_with_fence(generator: &NetherGenerator) -> NetherChunk {
        let mut body = NetherChunk::empty(generator.definition);
        body.set(0, 40, 0, generator.ids.nether_brick_fence_sides[0]);
        body.compact();
        body
    }

    #[test]
    fn join_reports_a_missed_neighbour_instead_of_panicking() {
        let generator = generator();
        let body = body_with_fence(&generator);
        assert!(generator.structure.joins_reach_neighbours(&body, &generator.ids, &generator.blocks));
        let none: [Option<Arc<NetherChunk>>; 9] = std::array::from_fn(|_| None);
        let outcome = generator.finish_structure((0, 0), body.clone(), &none);
        assert!(matches!(outcome, Err(PhaseError::UndeclaredRead { dependency: "StructureBodyPhase", .. })));
        let all: [Option<Arc<NetherChunk>>; 9] = std::array::from_fn(|_| Some(Arc::new(NetherChunk::empty(generator.definition))));
        assert!(generator.finish_structure((0, 0), body, &all).is_ok());
    }

    #[test]
    fn body_reports_a_missing_centre_instead_of_panicking() {
        let generator = generator();
        let none: [Option<Arc<NetherChunk>>; 25] = std::array::from_fn(|_| None);
        assert!(matches!(
            generator.structure_body((0, 0), &none, &[], &[]),
            Err(PhaseError::UndeclaredRead { dependency: "CarverPhase", .. })
        ));
    }

    #[test]
    fn carved_lookup_rejects_cells_outside_the_window() {
        assert_eq!(reach_two_index(0, 0), Some(12));
        assert_eq!(reach_two_index(3, 0), None);
        assert_eq!(reach_two_index(0, -3), None);
    }
}
