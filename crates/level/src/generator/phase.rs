use std::any::{Any, TypeId};
use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::generator::dimension::Generator;
use crate::generator::error::PhaseError;
use crate::generator::pos::ChunkPos;

pub type ErasedOutput = Arc<dyn Any + Send + Sync>;

pub const SAME_CELL: &[(i32, i32)] = &[(0, 0)];

#[derive(Clone, Debug)]
pub struct MailboxPush {
    pub mailbox_type: TypeId,
    pub target: ChunkPos,
    pub source: ChunkPos,
    pub hop: u32,
    pub value: ErasedOutput,
}

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
    type Mailbox: Send + Sync + 'static;

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
        }
    }
}

pub struct Requirement<G: Generator> {
    pub descriptor: PhaseDescriptor<G>,
    pub offsets: &'static [(i32, i32)],
    pub max_hops: Option<u32>,
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

pub struct PhaseInputs<G: Generator> {
    pub resolved: Vec<(PhaseId, ChunkPos, ErasedOutput)>,
    pub generator: std::marker::PhantomData<fn() -> G>,
    pub cell: ChunkPos,
    pub hop: u32,
    pub inbox: FxHashMap<(TypeId, ChunkPos, u32), Vec<MailboxEntry>>,
    pub mailbox_out: Vec<MailboxPush>,
}

#[derive(Clone)]
pub struct MailboxEntry {
    pub source: ChunkPos,
    pub value: ErasedOutput,
}



impl<G: Generator> PhaseInputs<G> {
    pub fn inbox<M: Phase<G>>(&self, cell: ChunkPos, hop: u32) -> Vec<(ChunkPos, Arc<M::Mailbox>)> {
        self.inbox.get(&(TypeId::of::<M>(), cell, hop))
            .map(|v| v.iter().filter_map(|e| {
                e.value.clone().downcast::<M::Mailbox>().ok().map(|v| (e.source, v))
            }).collect())
            .unwrap_or_default()
    }

    pub fn mail<M: Phase<G>>(&mut self, target: ChunkPos, value: M::Mailbox) {
        self.mailbox_out.push(MailboxPush {
            mailbox_type: TypeId::of::<M>(),
            target,
            source: self.cell,
            hop: self.hop,
            value: Arc::new(value) as ErasedOutput,
        });
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
