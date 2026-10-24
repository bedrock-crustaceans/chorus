use std::sync::Arc;
#[cfg(feature = "debug")]
use std::time::{Duration, Instant};

use rustc_hash::{FxHashMap, FxHashSet};
use slotmap::{SlotMap, new_key_type};

use crate::generator::dimension::Generator;
use crate::generator::error::PhaseFailure;
use crate::generator::phase::{ErasedOutput, PhaseDescriptor, PhaseId};
use crate::generator::pos::ChunkPos;

new_key_type! { pub struct NodeKey; }

pub type Outcome = Result<ErasedOutput, Arc<PhaseFailure>>;

pub enum NodeStatus {
    Pending { remaining: usize },
    Running,
    Done(Outcome),
}

pub struct Node<G: Generator> {
    pub reused: bool,
    pub descriptor: PhaseDescriptor<G>,
    pub cell: ChunkPos,
    pub hop: u32,
    pub order: u64,
    pub priority: u64,
    pub requested: bool,
    pub retained: Option<u64>,
    pub deps: Vec<NodeKey>,
    pub dependents: Vec<NodeKey>,
    pub status: NodeStatus,
    pub mailbox_out: Vec<(ChunkPos, ErasedOutput)>,
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
    pub pushes: u64,
    pub nodes: FxHashSet<NodeKey>,
}

impl Retention {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            stats: RetentionStats::default(),
            pushes: 0,
            nodes: FxHashSet::default(),
        }
    }
}

pub struct Graph<G: Generator> {
    pub nodes: SlotMap<NodeKey, Node<G>>,
    pub index: FxHashMap<(PhaseId, ChunkPos, u32), NodeKey>,
    pub retained: FxHashMap<PhaseId, Retention>,
}

impl<G: Generator> Graph<G> {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            index: FxHashMap::default(),
            retained: FxHashMap::default(),
        }
    }
}

impl<G: Generator> Default for Graph<G> {
    fn default() -> Self {
        Self::new()
    }
}
