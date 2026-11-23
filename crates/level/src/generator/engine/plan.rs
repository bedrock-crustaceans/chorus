use std::marker::PhantomData;
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::generator::dimension::Generator;
use crate::generator::error::{PhaseError, PhaseFailure};
use crate::generator::phase::{PhaseContext, PhaseHandle, PhaseId, Request};
use crate::generator::tasks::{Task, TaskKey, TaskStatus};

use super::*;
use glam::IVec2;

struct Entry<G: Generator> {
    handle: PhaseHandle<G>,
    cell: IVec2,
    hop: u32,
    demand_rank: Option<u64>,
    reader: Option<(IVec2, &'static [(i32, i32)])>,
}

struct Expansion<G: Generator> {
    dep_keys: Vec<(PhaseId, IVec2, u32)>,
    children: Vec<Entry<G>>,
    invalid: Option<PhaseError>,
}

enum Step<G: Generator> {
    Enter(Entry<G>),
    Finalize {
        key: TaskKey,
        dep_keys: Vec<(PhaseId, IVec2, u32)>,
        invalid: Option<PhaseError>,
    },
}

impl<G: Generator> Engine<G> {
    pub fn insert(&mut self, handle: PhaseHandle<G>, cell: IVec2, order: u64) {
        self.insert_demanded(handle, cell, order, None);
    }

    pub(super) fn insert_demanded(&mut self, handle: PhaseHandle<G>, cell: IVec2, order: u64, demand_rank: Option<u64>) {
        self.revision = self.revision.wrapping_add(1);
        let mut work = vec![Step::Enter(Entry {
            handle,
            cell,
            hop: 0,
            demand_rank,
            reader: None,
        })];
        while let Some(step) = work.pop() {
            match step {
                Step::Enter(entry) => self.enter(entry, order, &mut work),
                Step::Finalize { key, dep_keys, invalid } => self.finalize(key, dep_keys, invalid),
            }
        }
    }

    fn enter(&mut self, entry: Entry<G>, order: u64, work: &mut Vec<Step<G>>) {
        let Entry {
            handle,
            cell,
            hop,
            demand_rank,
            reader,
        } = entry;
        if let Some(existing) = self.key_at(handle.phase, cell, hop) {
            self.note_demand(existing, demand_rank);
            if !self.lacks_mail_for(existing, reader) {
                return;
            }
            self.close(existing);
        }

        let key = self.add_node(handle, cell, hop, order, demand_rank);
        let Expansion { dep_keys, children, invalid } = Self::expand(handle, cell, hop, demand_rank);
        if invalid.is_some() {
            work.push(Step::Finalize { key, dep_keys: Vec::new(), invalid });
        } else {
            work.push(Step::Finalize { key, dep_keys, invalid });
            work.extend(children.into_iter().map(Step::Enter));
        }
    }

    fn add_node(&mut self, handle: PhaseHandle<G>, cell: IVec2, hop: u32, order: u64, demand_rank: Option<u64>) -> TaskKey {
        let phase = self.graph.phases.slot(handle);
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        let key = self.graph.tasks.insert(Task {
            reused: false,
            phase,
            generator: PhantomData,
            cell,
            hop,
            sequence,
            order,
            priority: 0,
            demand_rank,
            requested: false,
            retained: None,
            deps: Vec::new(),
            dependents: Vec::new(),
            status: TaskStatus::Pending { remaining: 0 },
            settled_deps: None,
            resolving: false,
            #[cfg(feature = "debug")]
            started_at: None,
            #[cfg(feature = "debug")]
            duration: None,
        });
        self.graph.index.insert((handle.phase, cell, hop), key);
        key
    }

    fn expand(handle: PhaseHandle<G>, cell: IVec2, hop: u32, demand_rank: Option<u64>) -> Expansion<G> {
        let mut expansion = Expansion {
            dep_keys: Vec::new(),
            children: Vec::new(),
            invalid: None,
        };
        for dependency in (handle.dependencies)() {
            if dependency.if_dependent.is_some_and(|applies| !applies(cell)) {
                continue;
            }
            let depends_on_itself = dependency.handle.phase == handle.phase;
            let dependency_hop = match (depends_on_itself, dependency.max_hops) {
                (false, None) => 0,
                (true, Some(max_hops)) if hop < max_hops => hop + 1,
                (true, Some(_)) => continue,
                (true, None) => {
                    expansion.invalid = Some(PhaseError::UnboundedSelfDependency);
                    break;
                }
                (false, Some(_)) => {
                    expansion.invalid = Some(PhaseError::MaxHopsOnOtherPhase { dependency: dependency.handle.name });
                    break;
                }
            };

            for &(dx, dz) in dependency.offsets {
                let dependency_cell = cell + IVec2::new(dx, dz);
                if dependency.if_dependency.is_some_and(|applies| !applies(dependency_cell)) {
                    continue;
                }
                expansion.dep_keys.push((dependency.handle.phase, dependency_cell, dependency_hop));
                expansion.children.push(Entry {
                    handle: dependency.handle,
                    cell: dependency_cell,
                    hop: dependency_hop,
                    demand_rank,
                    reader: dependency.mail_for.map(|targets| (cell, targets)),
                });
            }
        }
        expansion
    }

    fn finalize(&mut self, key: TaskKey, dep_keys: Vec<(PhaseId, IVec2, u32)>, invalid: Option<PhaseError>) {
        if let Some(error) = invalid {
            let task = &mut self.graph.tasks[key];
            let failure = PhaseFailure {
                phase: self.graph.phases.name(task),
                cell: task.cell,
                error,
            };
            task.status = TaskStatus::Done(Err(Arc::new(failure)));
            return;
        }

        let mut deps = Vec::with_capacity(dep_keys.len());
        for dep_key in dep_keys {
            let dependency_key = self.graph.index[&dep_key];
            deps.push(dependency_key);
            self.depend(key, dependency_key);
        }

        let remaining = deps.iter().filter(|dependency_key| !matches!(self.graph.tasks[**dependency_key].status, TaskStatus::Done(_))).count();
        self.graph.tasks[key].deps = deps;

        if remaining == 0 {
            self.push_ready(key);
        } else {
            self.graph.tasks[key].status = TaskStatus::Pending { remaining };
        }
    }

    pub fn start(&mut self, key: TaskKey) -> Result<Dispatch<G>, Arc<PhaseFailure>> {
        self.revision = self.revision.wrapping_add(1);
        let task = &self.graph.tasks[key];
        let handle = self.graph.phases.handle(task);
        let cell = task.cell;
        let hop = task.hop;

        let context = loop {
            let context = match self.context(key, handle, cell, hop) {
                Ok(context) => context,
                Err(failure) => {
                    self.release_dependencies(key);
                    return Err(failure);
                }
            };
            let requests = (handle.resolve)(&self.generator, cell, &context);
            if requests.is_empty() {
                break context;
            }
            let known = self.graph.tasks[key].deps.len();
            let waiting = self.attach_requests(key, handle, requests)?;
            if waiting > 0 {
                self.graph.tasks[key].status = TaskStatus::Pending { remaining: waiting };
                return Ok(None);
            }
            if self.graph.tasks[key].deps.len() == known {
                break context;
            }
        };

        *self.started.entry(handle.name).or_default() += 1;
        self.graph.tasks[key].status = TaskStatus::Running;
        #[cfg(feature = "debug")]
        {
            self.graph.tasks[key].started_at = Some(Instant::now());
        }

        self.release_dependencies(key);

        Ok(Some((handle, cell, context)))
    }

    pub(super) fn context(&mut self, key: TaskKey, handle: PhaseHandle<G>, cell: IVec2, hop: u32) -> Result<PhaseContext<G>, Arc<PhaseFailure>> {
        let mut producers = std::mem::take(&mut self.producers);
        producers.clear();
        let task = &self.graph.tasks[key];
        let mut resolved = Vec::with_capacity(task.deps.len());
        let mut failed = None;
        for dependency_key in &task.deps {
            let dependency = &self.graph.tasks[*dependency_key];
            match &dependency.status {
                TaskStatus::Done(Ok(output)) => {
                    resolved.push((self.graph.phases.id(dependency), dependency.cell, output.clone()));
                    producers.insert((self.graph.phases.id(dependency), dependency.cell, dependency.hop), *dependency_key);
                }
                TaskStatus::Done(Err(failure)) => {
                    failed = Some(failure.clone());
                    break;
                }
                _ => unreachable!("dependency dispatched before it completed"),
            }
        }

        if let Some(failure) = failed {
            self.producers = producers;
            let error = PhaseError::DependencyFailed(failure);
            return Err(Arc::new(PhaseFailure { phase: handle.name, cell, error }));
        }

        let inbox = self.collect_mail(handle.phase, cell, hop, &producers);
        self.producers = producers;
        Ok(PhaseContext {
            resolved,
            generator: PhantomData,
            phase: handle.phase,
            inbox,
            outbox: Vec::new(),
        })
    }

    pub(super) fn release_dependencies(&mut self, key: TaskKey) {
        for i in 0..self.graph.tasks[key].deps.len() {
            let dependency_key = self.graph.tasks[key].deps[i];
            self.try_evict(dependency_key);
        }
    }

    pub(super) fn attach_requests(&mut self, key: TaskKey, handle: PhaseHandle<G>, requests: Vec<Request<G>>) -> Result<usize, Arc<PhaseFailure>> {
        let cell = self.graph.tasks[key].cell;
        let demand_rank = self.rank_of(&self.graph.tasks[key]);
        let mut widened = false;
        for request in &requests {
            let distance = (request.cell.x - cell.x).abs().max((request.cell.y - cell.y).abs());
            let known = self.edge_reach.entry((handle.phase, request.handle.phase)).or_insert(-1);
            if distance > *known {
                *known = distance;
                widened = true;
            }
        }
        if widened {
            self.settle_reach();
        }
        for request in requests {
            if self.key_at(request.handle.phase, request.cell, 0).is_none() {
                let order = self.next_order;
                self.next_order += 1;
                self.insert_demanded(request.handle, request.cell, order, Some(demand_rank));
            }
            let Some(dependency_key) = self.key_at(request.handle.phase, request.cell, 0) else { continue };
            self.note_demand(dependency_key, Some(demand_rank));
            if dependency_key == key || self.reaches(dependency_key, key) {
                let error = PhaseError::RequestCycle {
                    dependency: request.handle.name,
                    cell: request.cell,
                };
                return Err(Arc::new(PhaseFailure { phase: handle.name, cell, error }));
            }
            if self.graph.tasks[key].deps.contains(&dependency_key) {
                continue;
            }
            self.graph.tasks[key].deps.push(dependency_key);
            self.depend(key, dependency_key);
        }
        Ok(self.unfinished_dependencies(key))
    }

    pub(super) fn settle_reach(&mut self) {
        let phases: FxHashSet<PhaseId> = self.edge_reach.keys().flat_map(|&(from, to)| [from, to]).collect();
        let mut reach: FxHashMap<PhaseId, i32> = FxHashMap::default();
        for _ in 0..=phases.len() {
            let mut next: FxHashMap<PhaseId, i32> = FxHashMap::default();
            for (&(from, to), &distance) in &self.edge_reach {
                if from == to {
                    continue;
                }
                let through = reach.get(&from).copied().unwrap_or(0) + distance;
                let entry = next.entry(to).or_insert(0);
                *entry = (*entry).max(through);
            }
            if next == reach {
                break;
            }
            reach = next;
        }
        for (&(from, to), &distance) in &self.edge_reach {
            if from == to {
                *reach.entry(to).or_insert(0) += distance;
            }
        }
        if reach != self.demand_reach {
            self.demand_reach = reach;
            self.rebuild_regions();
        }
    }

    pub(super) fn unfinished_dependencies(&self, key: TaskKey) -> usize {
        self.graph.tasks[key]
            .deps
            .iter()
            .filter(|&&dependency| !matches!(self.graph.tasks[dependency].status, TaskStatus::Done(_)))
            .count()
    }

    pub(super) fn reaches(&self, from: TaskKey, target: TaskKey) -> bool {
        let mut stack = vec![from];
        let mut seen = FxHashSet::default();
        while let Some(key) = stack.pop() {
            if key == target {
                return true;
            }
            if !seen.insert(key) {
                continue;
            }
            let Some(task) = self.graph.tasks.get(key) else { continue };
            stack.extend(task.deps.iter().copied());
        }
        false
    }

    pub(super) fn depend(&mut self, key: TaskKey, dependency_key: TaskKey) {
        let dependency = &mut self.graph.tasks[dependency_key];
        dependency.dependents.push(key);
        if let Some(stamp) = dependency.retained.take()
            && let Some(retention) = self.graph.retained.get_mut(&self.graph.phases.id(dependency))
        {
            retention.entries.remove(&stamp);
            retention.stats.reuse_events += 1;
            if !dependency.reused {
                dependency.reused = true;
                retention.stats.nodes_reused += 1;
            }
        }
    }
}
