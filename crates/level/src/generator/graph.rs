use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

use rustc_hash::{FxHashMap, FxHashSet};
use slotmap::{SlotMap, new_key_type};

use crate::generator::dimension::Generator;
use crate::generator::error::PhaseFailure;
use crate::generator::phase::{ErasedOutput, PhaseDescriptor, PhaseId};
use crate::generator::pos::ChunkPos;

new_key_type! { pub(crate) struct NodeKey; }

pub(crate) type Outcome = Result<ErasedOutput, Arc<PhaseFailure>>;

pub(crate) enum NodeStatus {
    Pending { remaining: usize },
    Running,
    Done(Outcome),
}

pub(crate) struct Node<G: Generator> {
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
}

pub(crate) struct Retention {
    pub ceiling: usize,
    pub capacity: usize,
    pub pushes: u64,
    pub queue: VecDeque<(NodeKey, u64)>,
    pub ghosts: VecDeque<(ChunkPos, u32)>,
    pub ghost_set: FxHashSet<(ChunkPos, u32)>,
    pub window_start: Instant,
    pub window_max_depth: u64,
}

impl Retention {
    pub fn new(ceiling: usize) -> Self {
        Self {
            ceiling,
            capacity: 0,
            pushes: 0,
            queue: VecDeque::new(),
            ghosts: VecDeque::new(),
            ghost_set: FxHashSet::default(),
            window_start: Instant::now(),
            window_max_depth: 0,
        }
    }
}

pub(crate) struct Graph<G: Generator> {
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
