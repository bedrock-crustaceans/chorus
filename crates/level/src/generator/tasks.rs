use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::Arc;
#[cfg(feature = "debug")]
use std::time::{Duration, Instant};

use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};

use crate::generator::dimension::Generator;
use crate::generator::error::PhaseFailure;
use crate::generator::phase::{ErasedOutput, PhaseHandle, PhaseId};
use glam::IVec2;

new_key_type! { pub struct TaskKey; }

pub type Outcome = Result<ErasedOutput, Arc<PhaseFailure>>;

pub enum TaskStatus {
    Pending { remaining: usize },
    Running,
    Done(Outcome),
}

pub struct Task<G: Generator> {
    pub reused: bool,
    pub phase: u8,
    pub generator: PhantomData<fn() -> G>,
    pub cell: IVec2,
    pub hop: u32,
    pub sequence: u64,
    pub order: u64,
    pub priority: u64,
    pub demand_rank: Option<u64>,
    pub requested: bool,
    pub retained: Option<u64>,
    pub deps: Vec<TaskKey>,
    pub dependents: Vec<TaskKey>,
    pub status: TaskStatus,
    pub settled_deps: Option<usize>,
    pub resolving: bool,
    #[cfg(feature = "debug")]
    pub started_at: Option<Instant>,
    #[cfg(feature = "debug")]
    pub duration: Option<Duration>,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct RetentionStats {
    pub pushed: u64,
    pub nodes_reused: u64,
    pub reuse_events: u64,
    pub evicted_unused: u64,
    pub evicted_after_reuse: u64,
}

pub struct Retention {
    pub name: &'static str,
    pub stats: RetentionStats,
    pub entries: BTreeMap<u64, TaskKey>,
}

impl Retention {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            stats: RetentionStats::default(),
            entries: BTreeMap::new(),
        }
    }
}

#[derive(Default)]
pub struct TaskMail {
    pub out: Vec<(IVec2, ErasedOutput)>,
    pub pruned: Vec<IVec2>,
}

pub struct PhaseTable<G: Generator> {
    handles: Vec<PhaseHandle<G>>,
}

impl<G: Generator> PhaseTable<G> {
    pub fn slot(&mut self, handle: PhaseHandle<G>) -> u8 {
        let slot = match self.handles.iter().position(|known| known.phase == handle.phase) {
            Some(slot) => slot,
            None => {
                self.handles.push(handle);
                self.handles.len() - 1
            }
        };
        u8::try_from(slot).expect("a generator has at most 256 phases")
    }

    pub fn handle(&self, task: &Task<G>) -> PhaseHandle<G> {
        self.handles[task.phase as usize]
    }

    pub fn id(&self, task: &Task<G>) -> PhaseId {
        self.id_at(task.phase)
    }

    pub fn id_at(&self, slot: u8) -> PhaseId {
        self.handles[slot as usize].phase
    }

    pub fn name(&self, task: &Task<G>) -> &'static str {
        self.handles[task.phase as usize].name
    }

    pub fn retain(&self, task: &Task<G>) -> usize {
        self.handles[task.phase as usize].retain
    }
}

pub struct TaskGraph<G: Generator> {
    pub tasks: SlotMap<TaskKey, Task<G>>,
    pub phases: PhaseTable<G>,
    pub mail: FxHashMap<TaskKey, TaskMail>,
    pub index: FxHashMap<(PhaseId, IVec2, u32), TaskKey>,
    pub retained: FxHashMap<PhaseId, Retention>,
}

impl<G: Generator> TaskGraph<G> {
    pub fn new() -> Self {
        Self {
            tasks: SlotMap::with_key(),
            phases: PhaseTable { handles: Vec::new() },
            mail: FxHashMap::default(),
            index: FxHashMap::default(),
            retained: FxHashMap::default(),
        }
    }
}

impl<G: Generator> TaskGraph<G> {
    pub fn remove_task(&mut self, key: TaskKey) -> Option<Task<G>> {
        self.mail.remove(&key);
        self.tasks.remove(key)
    }
}

impl<G: Generator> Default for TaskGraph<G> {
    fn default() -> Self {
        Self::new()
    }
}
