use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::Instant;

use crate::generator::dimension::Generator;
use crate::generator::tasks::{Task, TaskKey, TaskStatus};

use super::*;
use glam::IVec2;

impl<G: Generator> Engine<G> {
    pub fn nearest_focus(&self, cell: IVec2) -> Option<f64> {
        nearest_distance(&self.focus_by_x, cell)
    }

    pub fn rank_at(&self, task: &Task<G>, distance: Option<f64>) -> u64 {
        let Some(distance) = distance else { return 0 };
        let reach = self.reach.get(&self.graph.phases.id(task)).copied().unwrap_or(0.0);
        ((distance - reach).max(0.0) * 16.0).min(u32::MAX as f64) as u64
    }

    pub(super) fn rank_of(&self, task: &Task<G>) -> u64 {
        let own = self.rank_at(task, self.nearest_focus(task.cell));
        task.demand_rank.map_or(own, |demand| own.min(demand))
    }

    pub(super) fn priority_of(&self, key: TaskKey) -> u64 {
        let task = &self.graph.tasks[key];
        (self.rank_of(task) << 32) | (task.order & u32::MAX as u64)
    }

    pub fn push_ready(&mut self, key: TaskKey) {
        self.enqueue(key);
    }

    pub fn enqueue(&mut self, key: TaskKey) {
        let priority = self.priority_of(key);
        self.graph.tasks[key].priority = priority;
        self.pending_dispatch.push(Reverse((priority, key)));
    }

    pub(super) fn note_demand(&mut self, key: TaskKey, demand_rank: Option<u64>) {
        let Some(demand_rank) = demand_rank else { return };
        if self.graph.tasks[key].demand_rank.is_some_and(|known| known <= demand_rank) {
            return;
        }
        self.graph.tasks[key].demand_rank = Some(demand_rank);
        if matches!(self.graph.tasks[key].status, TaskStatus::Pending { remaining: 0 }) && self.priority_of(key) != self.graph.tasks[key].priority {
            self.enqueue(key);
        }
    }

    pub fn set_focus(&mut self, focus: &[(i32, i32)], radius: i32) {
        let focus: Vec<IVec2> = focus.iter().map(|&(x, z)| IVec2::new(x, z)).collect();
        if focus == self.focus && radius == self.radius {
            return;
        }
        let jumped = focus.len() != self.focus.len() || radius < self.radius || focus.iter().zip(&self.focus).any(|(now, before)| now.x.abs_diff(before.x).max(now.y.abs_diff(before.y)) > 1);
        let steps: Vec<(IVec2, IVec2)> = self.focus.iter().copied().zip(focus.iter().copied()).filter(|(before, now)| before != now).collect();
        self.focus_by_x = focus.clone();
        self.focus_by_x.sort_unstable_by_key(|focus| focus.x);
        self.focus = focus;
        let radius_changed = radius != self.radius;
        self.radius = radius;
        if radius_changed {
            self.rebuild_regions();
        }
        self.abandon_unwanted();

        if jumped {
            self.sweep_retained();
        } else {
            for (from, to) in steps {
                self.release_left_behind(from, to);
            }
        }

        if jumped || self.last_rank.elapsed() >= RERANK_INTERVAL {
            self.last_rank = Instant::now();
            let entries = std::mem::take(&mut self.pending_dispatch).into_vec();
            let mut ranked = Vec::with_capacity(entries.len());
            for Reverse((priority, key)) in entries {
                let ready = self
                    .graph
                    .tasks
                    .get(key)
                    .is_some_and(|task| task.priority == priority && matches!(task.status, TaskStatus::Pending { remaining: 0 }));
                if ready {
                    let priority = self.priority_of(key);
                    self.graph.tasks[key].priority = priority;
                    ranked.push(Reverse((priority, key)));
                }
            }
            self.pending_dispatch = BinaryHeap::from(ranked);
        }
        self.revision = self.revision.wrapping_add(1);
    }
}

pub(super) fn nearest_distance(by_x: &[IVec2], cell: IVec2) -> Option<f64> {
    let split = by_x.partition_point(|focus| focus.x < cell.x);
    let mut best: Option<f64> = None;
    let mut visit = |focus: &IVec2| {
        let dx = (focus.x - cell.x) as f64;
        if best.is_some_and(|best| dx.abs() >= best) {
            return false;
        }
        let dz = (cell.y - focus.y) as f64;
        let distance = (dx * dx + dz * dz).sqrt();
        best = Some(best.map_or(distance, |best| best.min(distance)));
        true
    };
    by_x[split..].iter().all(&mut visit);
    by_x[..split].iter().rev().all(&mut visit);
    best
}
