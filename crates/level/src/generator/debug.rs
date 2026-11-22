use std::time::Duration;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::generator::phase::PhaseId;
use crate::generator::tasks::TaskStatus;
use glam::IVec2;

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
    pub cell: IVec2,
    pub hop: u32,
    pub order: u64,
    pub status: DebugStatus,
    pub duration: Option<Duration>,
    pub dependencies: Vec<(PhaseId, IVec2, u32)>,
}

#[derive(Clone, Debug)]
pub struct DebugDependency {
    pub phase: PhaseId,
    pub max_hops: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct DebugPhase {
    pub id: PhaseId,
    pub name: &'static str,
    pub retain: usize,
    pub dependencies: Vec<DebugDependency>,
}

pub trait DebugGraph {
    fn terminal(&self) -> PhaseId;
    fn revision(&self) -> u64;
    fn focus(&self) -> &[IVec2];
    fn completed(&self) -> &FxHashSet<IVec2>;
    fn hops(&self) -> &FxHashMap<PhaseId, u32>;
    fn in_flight(&self) -> usize;
    fn ready(&self) -> usize;
    fn phases(&self) -> Vec<DebugPhase>;
    fn node(&self, phase: PhaseId, cell: IVec2, hop: u32) -> Option<DebugNode>;
    fn for_each_node(&self, f: &mut dyn FnMut(DebugNode));
    fn nodes_at(&self, cell: IVec2) -> Vec<DebugNode>;
}

impl DebugStatus {
    pub fn of(status: &TaskStatus) -> Self {
        match status {
            TaskStatus::Pending { remaining } if *remaining == 0 => Self::Ready,
            TaskStatus::Pending { .. } => Self::Pending,
            TaskStatus::Running => Self::Running,
            TaskStatus::Done(Ok(_)) => Self::Done,
            TaskStatus::Done(Err(_)) => Self::Failed,
        }
    }
}
