use std::any::{Any, TypeId};
use std::sync::Arc;

use crate::generator::dimension::Generator;
use crate::generator::error::PhaseError;
use crate::generator::pos::ChunkPos;

pub type ErasedOutput = Arc<dyn Any + Send + Sync>;

pub const SAME_CELL: &[(i32, i32)] = &[(0, 0)];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhaseId(TypeId);

impl PhaseId {
    pub fn of<P: 'static>() -> Self {
        Self(TypeId::of::<P>())
    }

    pub fn type_id(&self) -> TypeId {
        self.0
    }
}

pub trait Phase<G: Generator>: 'static {
    type Output: Send + Sync + 'static;

    const RETAIN: usize = 0;

    fn requires() -> Vec<Requirement<G>> {
        Vec::new()
    }

    fn run(generator: &G, cell: ChunkPos, inputs: &mut PhaseInputs<G>) -> Result<Self::Output, PhaseError>;

    fn at(offsets: &'static [(i32, i32)]) -> Requirement<G>
    where
        Self: Sized,
    {
        Requirement {
            descriptor: PhaseDescriptor::of::<Self>(),
            offsets,
            max_hops: None,
            mail_for: None,
        }
    }
}

/// A phase that sends a value to other cells besides its own output. The value is addressed to a target cell, and a
/// consumer that depends on this phase with [`Mailing::at_with_mail`] receives the values addressed to the cells it asks for.
pub trait Mailing<G: Generator>: Phase<G> {
    type Mail: Send + Sync + 'static;

    /// Depends on this phase at `offsets` like [`Phase::at`], and also receives what those cells addressed to the cells at `targets`
    /// (offsets from the consumer), in the order of `offsets`.
    fn at_with_mail(offsets: &'static [(i32, i32)], targets: &'static [(i32, i32)]) -> Requirement<G>
    where
        Self: Sized,
    {
        Requirement {
            mail_for: Some(targets),
            ..Self::at(offsets)
        }
    }
}

pub struct Requirement<G: Generator> {
    pub descriptor: PhaseDescriptor<G>,
    pub offsets: &'static [(i32, i32)],
    pub max_hops: Option<u32>,
    pub mail_for: Option<&'static [(i32, i32)]>,
}

impl<G: Generator> Requirement<G> {
    pub fn max_hops(mut self, max_hops: u32) -> Self {
        self.max_hops = Some(max_hops);
        self
    }
}

pub struct PhaseDescriptor<G: Generator> {
    pub phase: PhaseId,
    pub name: &'static str,
    pub retain: usize,
    pub requires: fn() -> Vec<Requirement<G>>,
    pub run: fn(&G, ChunkPos, &mut PhaseInputs<G>) -> Result<ErasedOutput, PhaseError>,
}

impl<G: Generator> Clone for PhaseDescriptor<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G: Generator> Copy for PhaseDescriptor<G> {}

impl<G: Generator> PhaseDescriptor<G> {
    pub fn of<P: Phase<G>>() -> Self {
        Self {
            phase: PhaseId::of::<P>(),
            name: short_type_name::<P>(),
            retain: P::RETAIN,
            requires: P::requires,
            run: |generator, cell, inputs| P::run(generator, cell, inputs).map(|output| Arc::new(output) as ErasedOutput),
        }
    }
}

pub type Outbox = Vec<(ChunkPos, ErasedOutput)>;

pub struct InboxEntry {
    pub producer: PhaseId,
    pub target: ChunkPos,
    pub source: ChunkPos,
    pub value: ErasedOutput,
}

pub struct PhaseInputs<G: Generator> {
    pub resolved: Vec<(PhaseId, ChunkPos, ErasedOutput)>,
    pub generator: std::marker::PhantomData<fn() -> G>,
    pub phase: PhaseId,
    pub inbox: Vec<InboxEntry>,
    pub outbox: Outbox,
}

impl<G: Generator> PhaseInputs<G> {
    /// What the cells this phase depends on with mail addressed to `target`, as `(source cell, mail)` in the order the requirement lists them.
    pub fn mail<P: Mailing<G>>(&self, target: ChunkPos) -> impl Iterator<Item = (ChunkPos, &P::Mail)> {
        let producer = PhaseId::of::<P>();
        self.inbox
            .iter()
            .filter(move |entry| entry.producer == producer && entry.target == target)
            .filter_map(|entry| entry.value.downcast_ref::<P::Mail>().map(|mail| (entry.source, mail)))
    }

    /// Addresses `value` to `target`. Only the phase that is running may send, as itself.
    pub fn send<P: Mailing<G>>(&mut self, target: ChunkPos, value: P::Mail) {
        assert_eq!(PhaseId::of::<P>(), self.phase, "a phase can only send mail as itself");
        self.outbox.push((target, Arc::new(value) as ErasedOutput));
    }

    pub fn get<Q: Phase<G>>(&self, cell: ChunkPos) -> Result<Arc<Q::Output>, PhaseError> {
        self.try_get::<Q>(cell).ok_or_else(|| Self::undeclared::<Q>(cell))
    }

    pub fn try_get<Q: Phase<G>>(&self, cell: ChunkPos) -> Option<Arc<Q::Output>> {
        let phase = PhaseId::of::<Q>();
        let (_, _, output) = self.resolved.iter().find(|(id, at, _)| *id == phase && *at == cell)?;
        Some(Self::downcast::<Q>(output.clone()))
    }

    pub fn take<Q: Phase<G>>(&mut self, cell: ChunkPos) -> Result<Q::Output, PhaseError>
    where
        Q::Output: Clone,
    {
        let phase = PhaseId::of::<Q>();
        let index = self.resolved.iter().position(|(id, at, _)| *id == phase && *at == cell).ok_or_else(|| Self::undeclared::<Q>(cell))?;
        let (_, _, output) = self.resolved.swap_remove(index);
        Ok(Arc::try_unwrap(Self::downcast::<Q>(output)).unwrap_or_else(|shared| (*shared).clone()))
    }

    fn downcast<Q: Phase<G>>(output: ErasedOutput) -> Arc<Q::Output> {
        output.downcast::<Q::Output>().expect("phase output stored under the wrong phase id")
    }

    fn undeclared<Q: Phase<G>>(cell: ChunkPos) -> PhaseError {
        PhaseError::UndeclaredRead {
            dependency: std::any::type_name::<Q>(),
            cell,
        }
    }
}

fn short_type_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    name.rsplit("::").next().unwrap_or(name)
}
