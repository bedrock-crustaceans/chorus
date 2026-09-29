use std::any::TypeId;
use std::sync::Arc;

use crate::level::generator::dimension::Generator;
use crate::level::generator::pos::ChunkPos;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhaseId(TypeId);

impl PhaseId {
    pub fn of<P: 'static>() -> Self {
        Self(TypeId::of::<P>())
    }
}

pub struct PhaseInputs<'a, G: Generator> {
    pub(crate) resolved: &'a [((PhaseId, ChunkPos), G::Value)],
}

impl<'a, G: Generator> PhaseInputs<'a, G> {
    pub fn get<Q: PhaseValue<G>>(&self, cell: ChunkPos) -> Arc<Q::Output> {
        self.try_get::<Q>(cell).expect("dependency read outside its declared requirement")
    }

    pub fn try_get<Q: PhaseValue<G>>(&self, cell: ChunkPos) -> Option<Arc<Q::Output>> {
        let key = (PhaseId::of::<Q>(), cell);
        self.resolved.iter().find(|(k, _)| *k == key).and_then(|(_, value)| Q::unwrap(value))
    }
}

pub struct PhaseDescriptor<G: Generator> {
    pub(crate) phase: PhaseId,
    pub(crate) requires: fn() -> Vec<Requirement<G>>,
    pub(crate) run: fn(&G, ChunkPos, &PhaseInputs<G>) -> G::Value,
}

impl<G: Generator> Clone for PhaseDescriptor<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G: Generator> Copy for PhaseDescriptor<G> {}

impl<G: Generator> PhaseDescriptor<G> {
    pub fn of<P: PhaseValue<G>>() -> Self {
        Self {
            phase: PhaseId::of::<P>(),
            requires: P::requires,
            run: |generator, cell, inputs| P::wrap(Arc::new(P::run(generator, cell, inputs))),
        }
    }
}

pub struct Requirement<G: Generator> {
    pub(crate) descriptor: PhaseDescriptor<G>,
    pub(crate) cells: fn(ChunkPos) -> Vec<ChunkPos>,
    pub(crate) self_max_hops: Option<u32>,
}

impl<G: Generator> Clone for Requirement<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G: Generator> Copy for Requirement<G> {}

pub trait Phase<G: Generator>: 'static {
    type Output: Send + Sync + 'static;

    fn requires() -> Vec<Requirement<G>> {
        Vec::new()
    }

    fn run(generator: &G, cell: ChunkPos, inputs: &PhaseInputs<G>) -> Self::Output;
}

pub trait PhaseValue<G: Generator>: Phase<G> {
    fn wrap(output: Arc<Self::Output>) -> G::Value;
    fn unwrap(value: &G::Value) -> Option<Arc<Self::Output>>;
}

pub fn requirement<G: Generator, P: PhaseValue<G>>(cells: fn(ChunkPos) -> Vec<ChunkPos>) -> Requirement<G> {
    Requirement {
        descriptor: PhaseDescriptor::of::<P>(),
        cells,
        self_max_hops: None,
    }
}

pub fn self_requirement<G: Generator, P: PhaseValue<G>>(cells: fn(ChunkPos) -> Vec<ChunkPos>, max_hops: u32) -> Requirement<G> {
    Requirement {
        descriptor: PhaseDescriptor::of::<P>(),
        cells,
        self_max_hops: Some(max_hops),
    }
}

pub fn same_cell(cell: ChunkPos) -> Vec<ChunkPos> {
    vec![cell]
}
