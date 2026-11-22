use rustc_hash::{FxHashMap, FxHashSet};

use crate::generator::dimension::Generator;
use crate::generator::footprint::CellSet;
use crate::generator::phase::{PhaseHandle, PhaseId};
use crate::generator::tasks::{Retention, Task, TaskKey, TaskStatus};

use super::*;
use glam::IVec2;

pub struct RetentionSize {
    pub phase: &'static str,
    pub hop: u32,
    pub retained: usize,
    pub cap: usize,
    pub footprint_cells: usize,
    pub footprint_radius: f64,
}

impl<G: Generator> Engine<G> {
    pub fn is_interior(&self, phase: PhaseId, hop: u32, cell: IVec2) -> bool {
        let Some(footprint) = self.footprints.get(&(phase, hop)) else { return false };
        footprint.iter().all(|&(dx, dz)| self.cell_finished(cell + IVec2::new(dx, dz)))
    }

    pub fn sweep(&mut self, around: IVec2) {
        let phases: Vec<PhaseId> = self.graph.retained.keys().copied().collect();
        for phase in phases {
            let hops = self.hops.get(&phase).copied().unwrap_or(0);
            for hop in 0..=hops {
                let Some(footprint) = self.footprints.get(&(phase, hop)) else { continue };
                let interior: Vec<TaskKey> = footprint
                    .iter()
                    .filter_map(|&(dx, dz)| {
                        let cell = around - IVec2::new(dx, dz);
                        let key = self.key_at(phase, cell, hop)?;
                        (self.graph.tasks[key].retained.is_some() && self.is_interior(phase, hop, cell)).then_some(key)
                    })
                    .collect();
                for key in interior {
                    self.remove(key);
                }
            }
        }
    }

    pub fn try_evict(&mut self, key: TaskKey) {
        let Some(task) = self.graph.tasks.get(key) else { return };
        if !matches!(task.status, TaskStatus::Done(_)) {
            return;
        }

        let still_needed = task
            .dependents
            .iter()
            .any(|&dependent| matches!(self.graph.tasks.get(dependent), Some(task) if task.resolving || matches!(task.status, TaskStatus::Pending { .. })));
        if still_needed {
            return;
        }

        let retain = self.graph.phases.retain(task);
        if retain == 0 {
            self.remove(key);
            return;
        }
        if task.retained.is_some() {
            return;
        }
        if self.is_interior(self.graph.phases.id(task), task.hop, task.cell) || !self.in_region(self.graph.phases.id(task), task.hop, task.cell) {
            self.remove(key);
            return;
        }

        let task = &mut self.graph.tasks[key];
        let phase = self.graph.phases.id(task);
        let name = self.graph.phases.name(task);
        let retention = self.graph.retained.entry(phase).or_insert_with(|| Retention::new(name));
        let stamp = task.sequence;
        retention.entries.insert(stamp, key);
        retention.stats.pushed += 1;
        task.retained = Some(stamp);
        task.dependents.clear();
        self.prune_mail(key);
        self.enforce_cap(phase);
    }

    pub(super) fn close(&mut self, key: TaskKey) {
        if self.graph.tasks[key].retained.is_some() {
            self.evict_retained(key);
        } else {
            self.unindex(key);
        }
    }

    pub(super) fn unindex(&mut self, key: TaskKey) {
        let Some(task) = self.graph.tasks.get(key) else { return };
        let id = (self.graph.phases.id(task), task.cell, task.hop);
        if self.graph.index.get(&id) == Some(&key) {
            self.graph.index.remove(&id);
        }
    }

    pub(super) fn node_at(&self, phase: PhaseId, cell: IVec2, hop: u32) -> Option<&Task<G>> {
        self.graph.index.get(&(phase, cell, hop)).and_then(|&key| self.graph.tasks.get(key))
    }

    pub(super) fn key_at(&self, phase: PhaseId, cell: IVec2, hop: u32) -> Option<TaskKey> {
        self.graph.index.get(&(phase, cell, hop)).copied().filter(|&key| self.graph.tasks.contains_key(key))
    }

    pub(super) fn cell_finished(&self, cell: IVec2) -> bool {
        match self.node_at(self.terminal, cell, 0) {
            Some(task) => matches!(task.status, TaskStatus::Done(_)),
            None => self.completed.contains(&cell),
        }
    }

    pub(super) fn retention_cap(&self) -> usize {
        let view_chunks = std::f64::consts::PI * (self.radius as f64).powi(2);
        (self.retention_budget * view_chunks) as usize * self.focus.len()
    }

    pub(super) fn enforce_cap(&mut self, phase: PhaseId) {
        if self.focus.is_empty() {
            return;
        }
        let cap = self.retention_cap();
        loop {
            let Some(retention) = self.graph.retained.get_mut(&phase) else { return };
            if retention.entries.len() <= cap {
                return;
            }
            let Some((_, key)) = retention.entries.pop_first() else { return };
            self.evict_retained(key);
        }
    }

    pub fn retained_count(&self) -> usize {
        self.graph.retained.values().map(|retention| retention.entries.len()).sum()
    }

    pub fn retention_sizes(&self) -> Vec<RetentionSize> {
        let mut names = FxHashMap::default();
        let mut stack = vec![PhaseHandle::of::<G::Terminal>()];
        while let Some(handle) = stack.pop() {
            if names.insert(handle.phase, handle.name).is_some() {
                continue;
            }
            stack.extend((handle.dependencies)().into_iter().map(|dependency| dependency.handle));
        }
        let cap = self.retention_cap();
        let mut sizes: Vec<RetentionSize> = self
            .footprints
            .iter()
            .map(|(&(phase, hop), cells)| RetentionSize {
                phase: names.get(&phase).copied().unwrap_or("unknown"),
                hop,
                retained: self.graph.retained.get(&phase).map_or(0, |retention| retention.entries.len()),
                cap,
                footprint_cells: cells.len(),
                footprint_radius: cells.iter().map(|&(x, z)| ((x * x + z * z) as f64).sqrt()).fold(0.0, f64::max),
            })
            .collect();
        let declared: FxHashSet<PhaseId> = self.footprints.keys().map(|&(phase, _)| phase).collect();
        for (&phase, retention) in &self.graph.retained {
            if declared.contains(&phase) {
                continue;
            }
            sizes.push(RetentionSize {
                phase: retention.name,
                hop: 0,
                retained: retention.entries.len(),
                cap,
                footprint_cells: 1,
                footprint_radius: 0.0,
            });
        }
        sizes.sort_unstable_by_key(|size| (size.phase, size.hop));
        sizes
    }

    pub(super) fn rebuild_regions(&mut self) {
        let demand = self.radius + self.lookahead;
        self.regions = self
            .footprints
            .iter()
            .map(|(&key, cells)| (key, CellSet::new(cells, demand + self.demand_reach.get(&key.0).copied().unwrap_or(0))))
            .collect();
        self.unit_regions = self
            .demand_reach
            .values()
            .copied()
            .chain(std::iter::once(0))
            .map(|reach| (reach, CellSet::new(&[(0, 0)], demand + reach)))
            .collect();
    }

    pub fn in_region(&self, phase: PhaseId, hop: u32, cell: IVec2) -> bool {
        if self.focus.is_empty() {
            return true;
        }
        let region = match self.regions.get(&(phase, hop)) {
            Some(region) => region,
            None => match self.unit_regions.get(&self.demand_reach.get(&phase).copied().unwrap_or(0)) {
                Some(region) => region,
                None => return true,
            },
        };
        self.region_contains(region, cell)
    }

    pub(super) fn region_contains(&self, region: &CellSet, cell: IVec2) -> bool {
        let from = self.focus_by_x.partition_point(|focus| focus.x < cell.x - region.half());
        self.focus_by_x[from..]
            .iter()
            .take_while(|focus| focus.x <= cell.x + region.half())
            .any(|focus| region.contains(focus.x - cell.x, focus.y - cell.y))
    }

    pub fn sweep_retained(&mut self) {
        self.abandon_unwanted();
        let outside: Vec<TaskKey> = self
            .graph
            .retained
            .values()
            .flat_map(|retention| retention.entries.values().copied())
            .filter(|&key| self.graph.tasks.get(key).is_none_or(|task| !self.in_region(self.graph.phases.id(task), task.hop, task.cell)))
            .collect();
        for key in outside {
            self.evict_retained(key);
        }
        let remaining: Vec<TaskKey> = self.graph.retained.values().flat_map(|retention| retention.entries.values().copied()).collect();
        for key in remaining {
            self.prune_mail(key);
        }
    }

    pub(super) fn cells_left_behind(&self, phase: PhaseId, hop: u32, from: IVec2, to: IVec2) -> Vec<IVec2> {
        let Some(region) = self.regions.get(&(phase, hop)) else { return Vec::new() };
        let half = region.half();
        let (dx, dz) = (to.x - from.x, to.y - from.y);
        let mut left = Vec::new();
        for ox in -half..=half {
            for oz in -half..=half {
                if region.contains(ox, oz) && !region.contains(ox + dx, oz + dz) {
                    left.push(from - IVec2::new(ox, oz));
                }
            }
        }
        left
    }

    pub(super) fn release_left_behind(&mut self, from: IVec2, to: IVec2) {
        let retained_phases: Vec<PhaseId> = self.graph.retained.keys().copied().collect();
        for phase in retained_phases {
            let hops = self.hops.get(&phase).copied().unwrap_or(0);
            for hop in 0..=hops {
                let outside: Vec<TaskKey> = self
                    .cells_left_behind(phase, hop, from, to)
                    .into_iter()
                    .filter(|&cell| !self.in_region(phase, hop, cell))
                    .filter_map(|cell| self.graph.index.get(&(phase, cell, hop)).copied())
                    .filter(|&key| self.graph.tasks.get(key).is_some_and(|task| task.retained.is_some()))
                    .collect();
                for key in outside {
                    self.evict_retained(key);
                }
            }
        }

        let mut producers = Vec::new();
        for (&producer_phase, consumers) in &self.mail_consumers {
            let producer_hops = self.hops.get(&producer_phase).copied().unwrap_or(0);
            for consumer in consumers {
                let consumer_hops = self.hops.get(&consumer.phase).copied().unwrap_or(0);
                for reader_hop in 0..=consumer_hops {
                    for reader in self.cells_left_behind(consumer.phase, reader_hop, from, to) {
                        for &(ox, oz) in consumer.offsets {
                            let cell = reader + IVec2::new(ox, oz);
                            for hop in 0..=producer_hops {
                                if let Some(&key) = self.graph.index.get(&(producer_phase, cell, hop)) {
                                    producers.push(key);
                                }
                            }
                        }
                    }
                }
            }
        }
        for key in producers {
            if self.graph.tasks.get(key).is_some_and(|task| task.retained.is_some()) {
                self.prune_mail(key);
            }
        }
    }

    pub(super) fn evict_retained(&mut self, key: TaskKey) {
        let Some(task) = self.graph.tasks.get_mut(key) else { return };
        let stamp = task.retained.take();
        let (phase, reused) = (self.graph.phases.id(task), task.reused);
        if let Some(retention) = self.graph.retained.get_mut(&phase) {
            if let Some(stamp) = stamp {
                retention.entries.remove(&stamp);
            }
            if reused {
                retention.stats.evicted_after_reuse += 1;
            } else {
                retention.stats.evicted_unused += 1;
            }
        }
        self.unindex(key);
        self.to_release.push(key);
    }
}
