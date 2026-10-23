use std::time::Duration;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::generator::graph::NodeStatus;
use crate::generator::phase::PhaseId;
use crate::generator::pos::ChunkPos;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugStatus {
    Pending,
    Ready,
    Running,
    Done,
    Failed,
}

#[derive(Clone, Debug)]
pub struct DebugNode {
    pub phase: PhaseId,
    pub phase_name: &'static str,
    pub cell: ChunkPos,
    pub hop: u32,
    pub order: u64,
    pub status: DebugStatus,
    pub duration: Option<Duration>,
    pub dependencies: Vec<(PhaseId, ChunkPos, u32)>,
}

#[derive(Clone, Debug)]
pub struct DebugRequirement {
    pub phase: PhaseId,
    pub max_hops: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct DebugPhase {
    pub id: PhaseId,
    pub name: &'static str,
    pub retain: usize,
    pub requirements: Vec<DebugRequirement>,
}

pub trait DebugGraph {
    fn terminal(&self) -> PhaseId;
    fn revision(&self) -> u64;
    fn focus(&self) -> &[ChunkPos];
    fn completed(&self) -> &FxHashSet<ChunkPos>;
    fn hops(&self) -> &FxHashMap<PhaseId, u32>;
    fn in_flight(&self) -> usize;
    fn ready(&self) -> usize;
    fn phases(&self) -> Vec<DebugPhase>;
    fn node(&self, phase: PhaseId, cell: ChunkPos, hop: u32) -> Option<DebugNode>;
    fn for_each_node(&self, f: &mut dyn FnMut(DebugNode));
    fn nodes_at(&self, cell: ChunkPos) -> Vec<DebugNode>;
}

impl DebugStatus {
    pub fn of(status: &NodeStatus) -> Self {
        match status {
            NodeStatus::Pending { remaining } if *remaining == 0 => Self::Ready,
            NodeStatus::Pending { .. } => Self::Pending,
            NodeStatus::Running => Self::Running,
            NodeStatus::Done(Ok(_)) => Self::Done,
            NodeStatus::Done(Err(_)) => Self::Failed,
        }
    }
}
