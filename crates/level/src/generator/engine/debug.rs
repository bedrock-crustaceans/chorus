use rustc_hash::{FxHashMap, FxHashSet};

use crate::generator::dimension::Generator;
use crate::generator::phase::{PhaseHandle, PhaseId};

use super::*;
use glam::IVec2;

#[cfg(feature = "debug")]
impl<G: Generator> crate::generator::debug::DebugGraph for Engine<G> {
    fn terminal(&self) -> PhaseId {
        self.terminal
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn focus(&self) -> &[IVec2] {
        &self.focus
    }

    fn completed(&self) -> &FxHashSet<IVec2> {
        &self.completed
    }

    fn hops(&self) -> &FxHashMap<PhaseId, u32> {
        &self.hops
    }

    fn in_flight(&self) -> usize {
        self.in_flight
    }

    fn ready(&self) -> usize {
        self.pending_dispatch.len()
    }

    fn phases(&self) -> Vec<crate::generator::debug::DebugPhase> {
        use crate::generator::debug::{DebugDependency, DebugPhase};

        let mut phases = Vec::new();
        let mut seen = FxHashSet::default();
        let mut stack = vec![PhaseHandle::of::<G::Terminal>()];
        while let Some(handle) = stack.pop() {
            if !seen.insert(handle.phase) {
                continue;
            }
            let dependencies = (handle.dependencies)()
                .into_iter()
                .map(|dependency| {
                    if dependency.handle.phase != handle.phase {
                        stack.push(dependency.handle);
                    }
                    DebugDependency {
                        phase: dependency.handle.phase,
                        max_hops: dependency.max_hops,
                    }
                })
                .collect();
            phases.push(DebugPhase {
                id: handle.phase,
                name: handle.name,
                retain: handle.retain,
                dependencies,
            });
        }
        phases
    }

    fn node(&self, phase: PhaseId, cell: IVec2, hop: u32) -> Option<crate::generator::debug::DebugNode> {
        self.graph.index.get(&(phase, cell, hop)).and_then(|&key| {
            self.graph.tasks.get(key).map(|node| crate::generator::debug::DebugNode {
                phase: self.graph.phases.id(node),
                phase_name: self.graph.phases.name(node),
                cell: node.cell,
                hop: node.hop,
                order: node.order,
                status: crate::generator::debug::DebugStatus::of(&node.status),
                duration: node.duration,
                dependencies: node
                    .deps
                    .iter()
                    .filter_map(|&dep| self.graph.tasks.get(dep))
                    .map(|dep| (self.graph.phases.id(dep), dep.cell, dep.hop))
                    .collect(),
            })
        })
    }

    fn for_each_node(&self, f: &mut dyn FnMut(crate::generator::debug::DebugNode)) {
        for node in self.graph.tasks.values() {
            f(crate::generator::debug::DebugNode {
                phase: self.graph.phases.id(node),
                phase_name: self.graph.phases.name(node),
                cell: node.cell,
                hop: node.hop,
                order: node.order,
                status: crate::generator::debug::DebugStatus::of(&node.status),
                duration: node.duration,
                dependencies: node
                    .deps
                    .iter()
                    .filter_map(|&dep| self.graph.tasks.get(dep))
                    .map(|dep| (self.graph.phases.id(dep), dep.cell, dep.hop))
                    .collect(),
            });
        }
    }

    fn nodes_at(&self, cell: IVec2) -> Vec<crate::generator::debug::DebugNode> {
        let mut nodes = Vec::new();
        for info in self.phases() {
            let hops = self.hops.get(&info.id).copied().unwrap_or(0);
            for hop in 0..=hops {
                if let Some(node) = self.node(info.id, cell, hop) {
                    nodes.push(node);
                }
            }
        }
        nodes
    }
}
