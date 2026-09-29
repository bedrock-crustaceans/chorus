use crossbeam_channel::Sender;
use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};

use crate::level::generator::dimension::Generator;
use crate::level::generator::phase::{PhaseDescriptor, PhaseId};
use crate::level::generator::pos::ChunkPos;

new_key_type! { pub(crate) struct NodeKey; }

pub(crate) enum NodeStatus<G: Generator> {
    Pending { remaining: usize },
    Running,
    Done(G::Value),
}

pub(crate) struct Node<G: Generator> {
    pub descriptor: PhaseDescriptor<G>,
    pub cell: ChunkPos,
    pub hop: u32,
    pub deps: Vec<NodeKey>,
    pub dependents: Vec<NodeKey>,
    pub status: NodeStatus<G>,
    pub waiters: Vec<Sender<(ChunkPos, G::Value)>>,
}

pub(crate) struct Graph<G: Generator> {
    pub nodes: SlotMap<NodeKey, Node<G>>,
    pub index: FxHashMap<(PhaseId, ChunkPos, u32), NodeKey>,
}

impl<G: Generator> Graph<G> {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            index: FxHashMap::default(),
        }
    }
}
