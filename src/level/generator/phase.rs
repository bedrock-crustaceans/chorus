use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::sync::Arc;

use crate::level::generator::pos::ChunkPos;

pub(crate) type ErasedOutput = Arc<dyn Any + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhaseId(TypeId);

impl PhaseId {
    pub fn of<P: 'static>() -> Self {
        Self(TypeId::of::<P>())
    }
}

pub struct PhaseInputs<'a, G> {
    pub(crate) resolved: &'a [((PhaseId, ChunkPos), ErasedOutput)],
    pub(crate) marker: PhantomData<fn() -> G>,
}

impl<'a, G> PhaseInputs<'a, G> {
    pub fn get<Q: Phase<G>>(&self, cell: ChunkPos) -> Arc<Q::Output> {
        self.try_get::<Q>(cell).expect("dependency read outside its declared requirement")
    }

    pub fn try_get<Q: Phase<G>>(&self, cell: ChunkPos) -> Option<Arc<Q::Output>> {
        let key = (PhaseId::of::<Q>(), cell);
        self.resolved.iter().find(|(k, _)| *k == key).map(|(_, output)| output.clone().downcast::<Q::Output>().unwrap())
    }
}

pub struct PhaseDescriptor<G: 'static> {
    pub(crate) phase: PhaseId,
    pub(crate) requires: fn() -> Vec<Requirement<G>>,
    pub(crate) run: fn(&G, ChunkPos, &PhaseInputs<G>) -> ErasedOutput,
}

impl<G> Clone for PhaseDescriptor<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G> Copy for PhaseDescriptor<G> {}

impl<G: Send + Sync + 'static> PhaseDescriptor<G> {
    pub fn of<P: Phase<G>>() -> Self {
        Self {
            phase: PhaseId::of::<P>(),
            requires: P::requires,
            run: |generator, cell, inputs| Arc::new(P::run(generator, cell, inputs)),
        }
    }
}

pub struct Requirement<G: 'static> {
    pub(crate) descriptor: PhaseDescriptor<G>,
    pub(crate) cells: fn(ChunkPos) -> Vec<ChunkPos>,
}

impl<G> Clone for Requirement<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G> Copy for Requirement<G> {}

pub trait Phase<G>: 'static {
    type Output: Send + Sync + 'static;

    fn requires() -> Vec<Requirement<G>> {
        Vec::new()
    }

    fn run(generator: &G, cell: ChunkPos, inputs: &PhaseInputs<G>) -> Self::Output;
}

pub fn requirement<G: Send + Sync + 'static, P: Phase<G>>(cells: fn(ChunkPos) -> Vec<ChunkPos>) -> Requirement<G> {
    Requirement {
        descriptor: PhaseDescriptor::of::<P>(),
        cells,
    }
}

pub fn same_cell(cell: ChunkPos) -> Vec<ChunkPos> {
    vec![cell]
}
