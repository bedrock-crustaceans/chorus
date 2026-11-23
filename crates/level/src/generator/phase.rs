use std::any::{Any, TypeId};
use std::sync::Arc;

use crate::generator::dimension::Generator;
use crate::generator::error::PhaseError;
use glam::IVec2;

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

    const NAME: &'static str = "";

    fn dependencies() -> Vec<Dependency<G>> {
        Vec::new()
    }

    fn resolve(_generator: &G, _cell: IVec2, _inputs: &PhaseContext<G>) -> Vec<Request<G>> {
        Vec::new()
    }

    fn run(generator: &G, cell: IVec2, context: &mut PhaseContext<G>) -> Result<Self::Output, PhaseError>;
}

pub struct Request<G: Generator> {
    pub handle: PhaseHandle<G>,
    pub cell: IVec2,
}

impl<G: Generator> Request<G> {
    pub fn of<P: Phase<G>>(cell: IVec2) -> Self {
        Self { handle: PhaseHandle::of::<P>(), cell }
    }
}

/// A phase that sends a value to other cells besides its own output. The value is addressed to a target cell, and a
pub trait Mailing<G: Generator>: Phase<G> {
    type Mail: Send + Sync + 'static;
}

pub struct Dependency<G: Generator> {
    pub handle: PhaseHandle<G>,
    pub offsets: &'static [(i32, i32)],
    pub max_hops: Option<u32>,
    pub mail_for: Option<&'static [(i32, i32)]>,
    pub if_dependent: Option<fn(IVec2) -> bool>,
    pub if_dependency: Option<fn(IVec2) -> bool>,
    pub position_gate: bool,
}

impl<G: Generator> Dependency<G> {
    pub fn on<P: Phase<G>>(offsets: &'static [(i32, i32)]) -> Self {
        Self {
            handle: PhaseHandle::of::<P>(),
            offsets,
            max_hops: None,
            mail_for: None,
            if_dependent: None,
            if_dependency: None,
            position_gate: false,
        }
    }

    pub fn with_mail(mut self, targets: &'static [(i32, i32)]) -> Self {
        self.mail_for = Some(targets);
        self
    }

    pub fn max_hops(mut self, max_hops: u32) -> Self {
        self.max_hops = Some(max_hops);
        self
    }

    pub fn if_dependent(mut self, condition: fn(IVec2) -> bool) -> Self {
        self.if_dependent = Some(condition);
        self
    }

    pub fn if_dependent_position(mut self, condition: fn(IVec2) -> bool) -> Self {
        self.if_dependent = Some(condition);
        self.position_gate = true;
        self
    }

    pub fn if_dependency(mut self, condition: fn(IVec2) -> bool) -> Self {
        self.if_dependency = Some(condition);
        self
    }

    pub fn if_dependency_position(mut self, condition: fn(IVec2) -> bool) -> Self {
        self.if_dependency = Some(condition);
        self.position_gate = true;
        self
    }
}

pub struct PhaseHandle<G: Generator> {
    pub phase: PhaseId,
    pub name: &'static str,
    pub retain: usize,
    pub dependencies: fn() -> Vec<Dependency<G>>,
    pub resolve: fn(&G, IVec2, &PhaseContext<G>) -> Vec<Request<G>>,
    pub run: fn(&G, IVec2, &mut PhaseContext<G>) -> Result<ErasedOutput, PhaseError>,
}

impl<G: Generator> Clone for PhaseHandle<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G: Generator> Copy for PhaseHandle<G> {}

impl<G: Generator> PhaseHandle<G> {
    pub fn of<P: Phase<G>>() -> Self {
        Self {
            phase: PhaseId::of::<P>(),
            name: if P::NAME.is_empty() { short_type_name::<P>() } else { P::NAME },
            retain: P::RETAIN,
            dependencies: P::dependencies,
            resolve: P::resolve,
            run: |generator, cell, context| P::run(generator, cell, context).map(|output| Arc::new(output) as ErasedOutput),
        }
    }
}

pub type Outbox = Vec<(IVec2, ErasedOutput)>;

pub struct InboxEntry {
    pub producer: PhaseId,
    pub target: IVec2,
    pub source: IVec2,
    pub value: ErasedOutput,
}

pub struct PhaseContext<G: Generator> {
    pub resolved: Vec<(PhaseId, IVec2, ErasedOutput)>,
    pub generator: std::marker::PhantomData<fn() -> G>,
    pub phase: PhaseId,
    pub inbox: Vec<InboxEntry>,
    pub outbox: Outbox,
}

impl<G: Generator> PhaseContext<G> {
    pub fn mail<P: Mailing<G>>(&self, target: IVec2) -> impl Iterator<Item = (IVec2, &P::Mail)> {
        let producer = PhaseId::of::<P>();
        self.inbox
            .iter()
            .filter(move |entry| entry.producer == producer && entry.target == target)
            .filter_map(|entry| entry.value.downcast_ref::<P::Mail>().map(|mail| (entry.source, mail)))
    }

    /// Addresses `value` to `target`. Only the phase that is running may send, as itself.
    pub fn send<P: Mailing<G>>(&mut self, target: IVec2, value: P::Mail) {
        assert_eq!(PhaseId::of::<P>(), self.phase, "a phase can only send mail as itself");
        self.outbox.push((target, Arc::new(value) as ErasedOutput));
    }

    pub fn get<Q: Phase<G>>(&self, cell: IVec2) -> Result<Arc<Q::Output>, PhaseError> {
        self.try_get::<Q>(cell).ok_or_else(|| Self::undeclared::<Q>(cell))
    }

    pub fn try_get<Q: Phase<G>>(&self, cell: IVec2) -> Option<Arc<Q::Output>> {
        let phase = PhaseId::of::<Q>();
        let (_, _, output) = self.resolved.iter().find(|(id, at, _)| *id == phase && *at == cell)?;
        Some(Self::downcast::<Q>(output.clone()))
    }

    pub fn take<Q: Phase<G>>(&mut self, cell: IVec2) -> Result<Q::Output, PhaseError>
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

    fn undeclared<Q: Phase<G>>(cell: IVec2) -> PhaseError {
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
