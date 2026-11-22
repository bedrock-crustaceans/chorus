use std::cmp::Reverse;
use std::sync::Arc;

use bevy_tasks::TaskPool;

use crate::generator::dimension::Generator;
use crate::generator::error::PhaseFailure;
use crate::generator::phase::{Outbox, PhaseContext, PhaseHandle};
use crate::generator::tasks::{Outcome, TaskKey, TaskStatus};

use super::*;
use glam::IVec2;

impl<G: Generator> Engine<G> {
    pub fn execute(handle: PhaseHandle<G>, generator: &G, cell: IVec2, mut context: PhaseContext<G>) -> (Outcome, Outbox) {
        RUNNING_PHASE.with(|running| running.set(handle.name));
        let outcome = (handle.run)(generator, cell, &mut context).map_err(|error| Arc::new(PhaseFailure { phase: handle.name, cell, error }));
        RUNNING_PHASE.with(|running| running.set(""));
        (outcome, std::mem::take(&mut context.outbox))
    }

    pub fn complete(&mut self, key: TaskKey, outcome: Outcome) {
        self.revision = self.revision.wrapping_add(1);
        self.abandoned.remove(&key);
        let task = &mut self.graph.tasks[key];
        #[cfg(feature = "debug")]
        {
            task.duration = task.started_at.map(|started| started.elapsed());
        }
        let delivery = task.requested.then(|| (task.cell, outcome.clone()));
        task.requested = false;
        let task_phase = self.graph.phases.id(&self.graph.tasks[key]);
        if let Some((cell, outcome)) = delivery {
            self.deliver(task_phase, cell, outcome);
        }
        let task = &mut self.graph.tasks[key];
        let finished_terminal = (self.graph.phases.id(task) == self.terminal).then_some(task.cell);
        task.status = TaskStatus::Done(outcome);
        if let Some(cell) = finished_terminal {
            self.completed.insert(cell);
        }

        for i in 0..self.graph.tasks[key].dependents.len() {
            let dependent_key = self.graph.tasks[key].dependents[i];
            let ready = match self.graph.tasks.get_mut(dependent_key).map(|dependent| &mut dependent.status) {
                Some(TaskStatus::Pending { remaining }) => {
                    *remaining -= 1;
                    *remaining == 0
                }
                _ => false,
            };
            if ready {
                self.push_ready(dependent_key);
            }
        }

        self.release_mail_read_by(key);
        self.try_evict(key);
        if let Some(cell) = finished_terminal {
            self.sweep(cell);
        }
    }

    pub fn handle(&mut self, completion: TaskEvent<G>) {
        match completion {
            TaskEvent::Resolved(..) => unreachable!("resolve jobs are only spawned with a pool"),
            TaskEvent::Finished(key, outcome, pushes) => {
                self.in_flight -= 1;

                if outcome.is_ok() {
                    self.store_mail(key, pushes);
                }
                self.complete(key, outcome);
            }
            TaskEvent::Panicked(key) => {
                let task = &self.graph.tasks[key];
                panic!("{} at ({}, {}) panicked on a worker thread", self.graph.phases.name(task), task.cell.x, task.cell.y);
            }
        }
    }

    pub fn handle_with(&mut self, completion: TaskEvent<G>, pool: &TaskPool) {
        let TaskEvent::Resolved(key, requests, context) = completion else {
            self.handle(completion);
            return;
        };
        self.revision = self.revision.wrapping_add(1);
        self.in_flight -= 1;
        let task = &mut self.graph.tasks[key];
        task.resolving = false;
        let handle = self.graph.phases.handle(task);
        let cell = task.cell;
        let known = task.deps.len();
        if requests.is_empty() {
            task.settled_deps = Some(known);
            self.launch(key, handle, cell, context, pool);
            return;
        }
        let waiting = match self.attach_requests(key, handle, requests) {
            Ok(waiting) => waiting,
            Err(failure) => {
                self.release_dependencies(key);
                self.complete(key, Err(failure));
                return;
            }
        };
        let task = &mut self.graph.tasks[key];
        if waiting > 0 {
            task.status = TaskStatus::Pending { remaining: waiting };
        } else if task.deps.len() == known {
            task.settled_deps = Some(known);
            self.launch(key, handle, cell, context, pool);
        } else {
            task.status = TaskStatus::Pending { remaining: 0 };
            self.push_ready(key);
        }
    }

    pub(super) fn dispatch(&mut self, key: TaskKey, pool: &TaskPool) {
        self.revision = self.revision.wrapping_add(1);
        let task = &self.graph.tasks[key];
        let handle = self.graph.phases.handle(task);
        let cell = task.cell;
        let hop = task.hop;
        let context = match self.context(key, handle, cell, hop) {
            Ok(context) => context,
            Err(failure) => {
                self.release_dependencies(key);
                self.complete(key, Err(failure));
                return;
            }
        };
        let task = &mut self.graph.tasks[key];
        if task.settled_deps == Some(task.deps.len()) {
            self.launch(key, handle, cell, context, pool);
            return;
        }
        task.status = TaskStatus::Running;
        task.resolving = true;
        self.in_flight += 1;
        let generator = self.generator.clone();
        let completions = self.completions_tx.clone();
        pool.spawn(async move {
            let report = PanicGuard { key, completions };
            let requests = (handle.resolve)(&generator, cell, &context);
            let _ = report.completions.send(TaskEvent::Resolved(key, requests, context));
        })
        .detach();
    }

    pub(super) fn launch(&mut self, key: TaskKey, handle: PhaseHandle<G>, cell: IVec2, context: PhaseContext<G>, pool: &TaskPool) {
        *self.started.entry(handle.name).or_default() += 1;
        self.graph.tasks[key].status = TaskStatus::Running;
        #[cfg(feature = "debug")]
        {
            self.graph.tasks[key].started_at = Some(Instant::now());
        }

        self.release_dependencies(key);

        let generator = self.generator.clone();
        let completions = self.completions_tx.clone();
        self.in_flight += 1;
        pool.spawn(async move {
            let report = PanicGuard { key, completions };
            let (outcome, pushes) = Self::execute(handle, &generator, cell, context);
            let _ = report.completions.send(TaskEvent::Finished(key, outcome, pushes));
        })
        .detach();
    }

    pub fn fill_pool(&mut self, pool: &TaskPool) {
        while self.in_flight < pool.thread_num() * 2 {
            let Some(key) = self.pop_ready() else { break };
            self.dispatch(key, pool);
        }
    }

    pub fn pop_ready(&mut self) -> Option<TaskKey> {
        while let Some(Reverse((priority, key))) = self.pending_dispatch.pop() {
            if self
                .graph
                .tasks
                .get(key)
                .is_some_and(|task| task.priority == priority && matches!(task.status, TaskStatus::Pending { remaining: 0 }))
            {
                return Some(key);
            }
        }
        None
    }
}
