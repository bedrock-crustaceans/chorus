use std::collections::HashMap;

use crossbeam_channel::Sender;
use slotmap::{SlotMap, new_key_type};

use crate::level::generator::phase::{ErasedOutput, PhaseDescriptor, PhaseId};
use crate::level::generator::pos::ChunkPos;

new_key_type! { pub(crate) struct NodeKey; }

pub(crate) enum NodeStatus {
    Pending { remaining: usize },
    Running,
    Done(ErasedOutput),
}

pub(crate) struct Node<G: 'static> {
    pub descriptor: PhaseDescriptor<G>,
    pub cell: ChunkPos,
    pub deps: Vec<NodeKey>,
    pub dependents: Vec<NodeKey>,
    pub status: NodeStatus,
    pub waiters: Vec<Sender<(ChunkPos, ErasedOutput)>>,
}

pub(crate) struct Graph<G: 'static> {
    pub nodes: SlotMap<NodeKey, Node<G>>,
    pub index: HashMap<(PhaseId, ChunkPos), NodeKey>,
}

impl<G: 'static> Graph<G> {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            index: HashMap::new(),
        }
    }
}
