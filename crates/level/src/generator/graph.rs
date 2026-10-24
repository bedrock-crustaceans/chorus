use std::collections::VecDeque;
use std::marker::PhantomData;
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
    pub phase: u8,
    pub generator: PhantomData<fn() -> G>,
    pub cell: ChunkPos,
    pub hop: u32,
    pub order: u64,
    pub priority: u64,
    pub requested: bool,
    pub retained: Option<u64>,
    pub deps: Vec<NodeKey>,
    pub dependents: Vec<NodeKey>,
    pub status: NodeStatus,
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
    pub queue: VecDeque<(NodeKey, u64)>,
}

impl Retention {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            stats: RetentionStats::default(),
            pushes: 0,
            nodes: FxHashSet::default(),
            queue: VecDeque::new(),
        }
    }
}

#[derive(Default)]
pub struct NodeMail {
    pub out: Vec<(ChunkPos, ErasedOutput)>,
    pub pruned: Vec<ChunkPos>,
}

pub struct PhaseTable<G: Generator> {
    descriptors: Vec<PhaseDescriptor<G>>,
}

impl<G: Generator> PhaseTable<G> {
    pub fn slot(&mut self, descriptor: PhaseDescriptor<G>) -> u8 {
        let slot = match self.descriptors.iter().position(|known| known.phase == descriptor.phase) {
            Some(slot) => slot,
            None => {
                self.descriptors.push(descriptor);
                self.descriptors.len() - 1
            }
        };
        u8::try_from(slot).expect("a generator has at most 256 phases")
    }

    pub fn descriptor(&self, node: &Node<G>) -> PhaseDescriptor<G> {
        self.descriptors[node.phase as usize]
    }

    pub fn id(&self, node: &Node<G>) -> PhaseId {
        self.id_at(node.phase)
    }

    pub fn id_at(&self, slot: u8) -> PhaseId {
        self.descriptors[slot as usize].phase
    }

    pub fn name(&self, node: &Node<G>) -> &'static str {
        self.descriptors[node.phase as usize].name
    }

    pub fn retain(&self, node: &Node<G>) -> usize {
        self.descriptors[node.phase as usize].retain
    }
}

pub struct Graph<G: Generator> {
    pub nodes: SlotMap<NodeKey, Node<G>>,
    pub phases: PhaseTable<G>,
    pub mail: FxHashMap<NodeKey, NodeMail>,
    pub index: FxHashMap<(PhaseId, ChunkPos, u32), NodeKey>,
    pub retained: FxHashMap<PhaseId, Retention>,
}

impl<G: Generator> Graph<G> {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            phases: PhaseTable { descriptors: Vec::new() },
            mail: FxHashMap::default(),
            index: FxHashMap::default(),
            retained: FxHashMap::default(),
        }
    }
}

impl<G: Generator> Graph<G> {
    pub fn remove_node(&mut self, key: NodeKey) -> Option<Node<G>> {
        self.mail.remove(&key);
        self.nodes.remove(key)
    }
}

impl<G: Generator> Default for Graph<G> {
    fn default() -> Self {
        Self::new()
    }
}
