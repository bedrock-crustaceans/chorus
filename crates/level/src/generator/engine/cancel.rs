use crate::generator::dimension::Generator;
use crate::generator::phase::PhaseHandle;
use crate::generator::tasks::{TaskKey, TaskStatus};

use super::*;
use glam::IVec2;

impl<G: Generator> Engine<G> {
    pub fn remove(&mut self, key: TaskKey) {
        self.revision = self.revision.wrapping_add(1);
        self.abandoned.remove(&key);
        let task = &self.graph.tasks[key];
        if let Some(stamp) = task.retained
            && let Some(retention) = self.graph.retained.get_mut(&self.graph.phases.id(task))
        {
            retention.entries.remove(&stamp);
        }
        self.unindex(key);
        if let Some(task) = self.graph.remove_task(key)
            && let TaskStatus::Done(outcome) = task.status
            && self.graph.phases.id_at(task.phase) != self.terminal
        {
            self.discarded.push(outcome);
        }
    }

    pub fn release_one(&mut self) -> bool {
        if let Some(key) = self.to_release.pop() {
            if let Some(task) = self.graph.remove_task(key)
                && let TaskStatus::Done(outcome) = task.status
            {
                self.discarded.push(outcome);
            }
            return !self.to_release.is_empty() || !self.discarded.is_empty();
        }
        self.discarded.pop();
        !self.discarded.is_empty()
    }

    pub fn cancel(&mut self, cell: IVec2) {
        let root = PhaseHandle::of::<G::Terminal>();
        let Some(key) = self.key_at(root.phase, cell, 0) else { return };
        let task = &mut self.graph.tasks[key];
        task.requested = false;
        if matches!(task.status, TaskStatus::Done(_)) {
            self.try_evict(key);
        } else {
            self.abandoned.insert(key);
        }
    }

    pub fn abandon_unwanted(&mut self) {
        let furthest = self.extent + (self.radius + self.lookahead) as f64;
        let doomed: Vec<TaskKey> = self
            .abandoned
            .iter()
            .copied()
            .filter(|&key| {
                self.graph
                    .tasks
                    .get(key)
                    .is_none_or(|task| task.requested || self.nearest_focus(task.cell).is_none_or(|distance| distance > furthest))
            })
            .collect();
        for key in doomed {
            self.abandoned.remove(&key);
            if self.graph.tasks.get(key).is_some_and(|task| !task.requested) {
                self.release_pipeline(key);
            }
        }
    }

    pub(super) fn release_pipeline(&mut self, key: TaskKey) {
        let mut unneeded = vec![key];
        while let Some(key) = unneeded.pop() {
            let Some(task) = self.graph.tasks.get(key) else { continue };
            if !matches!(task.status, TaskStatus::Pending { .. }) {
                self.try_evict(key);
                continue;
            }
            let still_needed = task.requested
                || task
                    .dependents
                    .iter()
                    .any(|&dependent| matches!(self.graph.tasks.get(dependent), Some(task) if matches!(task.status, TaskStatus::Pending { .. } | TaskStatus::Running)));
            if still_needed {
                continue;
            }

            unneeded.extend(task.deps.iter().copied());
            self.remove(key);
        }
    }
}
