use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bevy_tasks::{AsyncComputeTaskPool, TaskPool};
use crossbeam_channel::{Receiver, Sender};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::chunk::Chunk;
use crate::generator::dimension::{GenerationStats, Generator, WorldGenerator};
use crate::generator::error::{PhaseError, PhaseFailure};
use crate::generator::graph::{Graph, Node, NodeKey, NodeStatus, Outcome, Retention};
use crate::generator::phase::{PhaseDescriptor, PhaseId, PhaseInputs};
use crate::generator::pos::ChunkPos;

const RETENTION_WINDOW: Duration = Duration::from_secs(5);
const REPORT_INTERVAL: Duration = Duration::from_secs(30);
const COMPLETION_WAIT: Duration = Duration::from_millis(1);

pub enum Completion {
    Finished(NodeKey, Outcome),
    Panicked(NodeKey),
}

struct PanicReport {
    key: NodeKey,
    completions: Sender<Completion>,
}

impl Drop for PanicReport {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let _ = self.completions.send(Completion::Panicked(self.key));
        }
    }
}

pub struct PhaseGraph<G: Generator> {
    pub generator: Arc<G>,
    pub graph: Graph<G>,
    pub completions_tx: Sender<Completion>,
    pub completions_rx: Receiver<Completion>,
    pub pending_dispatch: BinaryHeap<Reverse<(u64, NodeKey)>>,
    pub next_order: u64,
    pub focus: Vec<ChunkPos>,
    pub focus_by_x: Vec<ChunkPos>,
    pub reach: FxHashMap<PhaseId, f64>,
    pub hops: FxHashMap<PhaseId, u32>,
    pub terminal: PhaseId,
    pub completed: FxHashSet<ChunkPos>,
    pub started: FxHashMap<&'static str, u64>,
    pub last_report: Instant,
    pub in_flight: usize,
    pub finished: Vec<(ChunkPos, Outcome)>,
    pub discarded: Vec<Outcome>,
    pub revision: u64,
}

impl<G: Generator> PhaseGraph<G> {
    pub fn new(generator: G) -> Self {
        let (completions_tx, completions_rx) = crossbeam_channel::unbounded();
        let (reach, hops) = phase_shape::<G>();

        Self {
            generator: Arc::new(generator),
            graph: Graph::new(),
            completions_tx,
            completions_rx,
            pending_dispatch: BinaryHeap::new(),
            next_order: 0,
            focus: Vec::new(),
            focus_by_x: Vec::new(),
            reach,
            hops,
            terminal: PhaseId::of::<G::Terminal>(),
            completed: FxHashSet::default(),
            started: FxHashMap::default(),
            last_report: Instant::now(),
            in_flight: 0,
            finished: Vec::new(),
            discarded: Vec::new(),
            revision: 0,
        }
    }

    pub fn request(&mut self, cell: ChunkPos) {
        let root = PhaseDescriptor::of::<G::Terminal>();
        let root_key = (root.phase, cell, 0);

        if let Some(&key) = self.graph.index.get(&root_key) {
            let node = &mut self.graph.nodes[key];
            match &node.status {
                NodeStatus::Done(outcome) => self.finished.push((cell, outcome.clone())),
                _ => node.requested = true,
            }
            return;
        }

        let order = self.next_order;
        self.next_order += 1;
        self.insert(root, cell, order);

        let key = self.graph.index[&root_key];
        let node = &mut self.graph.nodes[key];
        match &node.status {
            NodeStatus::Done(outcome) => {
                self.finished.push((cell, outcome.clone()));
                self.try_evict(key);
            }
            _ => node.requested = true,
        }
    }

    pub fn insert(&mut self, descriptor: PhaseDescriptor<G>, cell: ChunkPos, order: u64) {
        self.revision = self.revision.wrapping_add(1);

        enum Step<G: Generator> {
            Enter {
                descriptor: PhaseDescriptor<G>,
                cell: ChunkPos,
                hop: u32,
            },
            Finalize {
                key: NodeKey,
                dep_keys: Vec<(PhaseId, ChunkPos, u32)>,
                invalid: Option<PhaseError>,
            },
        }

        let mut work = vec![Step::Enter { descriptor, cell, hop: 0 }];

        while let Some(step) = work.pop() {
            match step {
                Step::Enter { descriptor, cell, hop } => {
                    let map_key = (descriptor.phase, cell, hop);
                    if self.graph.index.contains_key(&map_key) {
                        continue;
                    }

                    if let Some(retention) = self.graph.retained.get_mut(&descriptor.phase)
                        && retention.ghost_set.remove(&(cell, hop))
                    {
                        retention.capacity = (retention.capacity + 1).min(retention_ceiling(descriptor.retain, self.focus.len()));
                        retention.stats.ghost_hits += 1;
                    }

                    let key = self.graph.nodes.insert(Node {
                        reused: false,
                        descriptor,
                        cell,
                        hop,
                        order,
                        priority: 0,
                        requested: false,
                        retained: None,
                        deps: Vec::new(),
                        dependents: Vec::new(),
                        status: NodeStatus::Pending { remaining: 0 },
                        #[cfg(feature = "debug")]
                        started_at: None,
                        #[cfg(feature = "debug")]
                        duration: None,
                    });
                    self.graph.index.insert(map_key, key);

                    let mut dep_keys = Vec::new();
                    let mut child_enters = Vec::new();
                    let mut invalid = None;
                    for requirement in (descriptor.requires)() {
                        let requires_itself = requirement.descriptor.phase == descriptor.phase;
                        let dependency_hop = match (requires_itself, requirement.max_hops) {
                            (false, None) => 0,
                            (true, Some(max_hops)) if hop < max_hops => hop + 1,
                            (true, Some(_)) => continue,
                            (true, None) => {
                                invalid = Some(PhaseError::UnboundedSelfRequirement);
                                break;
                            }
                            (false, Some(_)) => {
                                invalid = Some(PhaseError::MaxHopsOnOtherPhase {
                                    requirement: requirement.descriptor.name,
                                });
                                break;
                            }
                        };

                        for &(dx, dz) in requirement.offsets {
                            let dependency_cell = ChunkPos::new(cell.x + dx, cell.z + dz);
                            dep_keys.push((requirement.descriptor.phase, dependency_cell, dependency_hop));
                            child_enters.push(Step::Enter {
                                descriptor: requirement.descriptor,
                                cell: dependency_cell,
                                hop: dependency_hop,
                            });
                        }
                    }

                    if invalid.is_some() {
                        work.push(Step::Finalize { key, dep_keys: Vec::new(), invalid });
                    } else {
                        work.push(Step::Finalize { key, dep_keys, invalid });
                        work.extend(child_enters);
                    }
                }
                Step::Finalize { key, dep_keys, invalid } => {
                    if let Some(error) = invalid {
                        let node = &mut self.graph.nodes[key];
                        let failure = PhaseFailure {
                            phase: node.descriptor.name,
                            cell: node.cell,
                            error,
                        };
                        node.status = NodeStatus::Done(Err(Arc::new(failure)));
                        continue;
                    }

                    let mut deps = Vec::with_capacity(dep_keys.len());
                    for dep_key in dep_keys {
                        let dependency_key = self.graph.index[&dep_key];
                        deps.push(dependency_key);
                        let dependency = &mut self.graph.nodes[dependency_key];
                        dependency.dependents.push(key);
                        if let Some(stamp) = dependency.retained.take()
                            && let Some(retention) = self.graph.retained.get_mut(&dependency.descriptor.phase)
                        {
                            retention.window_max_depth = retention.window_max_depth.max(retention.pushes - stamp);
                            retention.stats.reuse_events += 1;
                            if !dependency.reused {
                                dependency.reused = true;
                                retention.stats.nodes_reused += 1;
                            }
                        }
                    }

                    let remaining = deps.iter().filter(|dependency_key| !matches!(self.graph.nodes[**dependency_key].status, NodeStatus::Done(_))).count();
                    self.graph.nodes[key].deps = deps;

                    if remaining == 0 {
                        self.push_ready(key);
                    } else {
                        self.graph.nodes[key].status = NodeStatus::Pending { remaining };
                    }
                }
            }
        }
    }

    pub fn nearest_focus(&self, cell: ChunkPos) -> Option<f64> {
        nearest_distance(&self.focus_by_x, cell)
    }

    pub fn rank_at(&self, node: &Node<G>, distance: Option<f64>) -> u64 {
        let Some(distance) = distance else { return 0 };
        let reach = self.reach.get(&node.descriptor.phase).copied().unwrap_or(0.0);
        ((distance - reach).max(0.0) * 16.0).min(u32::MAX as f64) as u64
    }

    pub fn push_ready(&mut self, key: NodeKey) {
        let distance = self.nearest_focus(self.graph.nodes[key].cell);
        self.enqueue(key, distance);
    }

    pub fn enqueue(&mut self, key: NodeKey, distance: Option<f64>) {
        let node = &self.graph.nodes[key];
        let priority = (self.rank_at(node, distance) << 32) | (node.order & u32::MAX as u64);
        self.graph.nodes[key].priority = priority;
        self.pending_dispatch.push(Reverse((priority, key)));
    }

    pub fn set_focus(&mut self, focus: &[(i32, i32)]) {
        let focus: Vec<ChunkPos> = focus.iter().map(|&(x, z)| ChunkPos::new(x, z)).collect();
        if focus == self.focus {
            return;
        }
        self.focus_by_x = focus.clone();
        self.focus_by_x.sort_unstable_by_key(|focus| focus.x);
        self.focus = focus;
        let entries = std::mem::take(&mut self.pending_dispatch).into_vec();
        let mut ranked = Vec::with_capacity(entries.len());
        for Reverse((priority, key)) in entries {
            let ready = self
                .graph
                .nodes
                .get(key)
                .is_some_and(|node| node.priority == priority && matches!(node.status, NodeStatus::Pending { remaining: 0 }));
            if ready {
                let node = &self.graph.nodes[key];
                let distance = self.nearest_focus(node.cell);
                let priority = (self.rank_at(node, distance) << 32) | (node.order & u32::MAX as u64);
                self.graph.nodes[key].priority = priority;
                ranked.push(Reverse((priority, key)));
            }
        }
        self.pending_dispatch = BinaryHeap::from(ranked);
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn start(&mut self, key: NodeKey) -> Result<(PhaseDescriptor<G>, ChunkPos, PhaseInputs<G>), Arc<PhaseFailure>> {
        self.revision = self.revision.wrapping_add(1);
        let node = &self.graph.nodes[key];
        let descriptor = node.descriptor;
        let cell = node.cell;
        *self.started.entry(descriptor.name).or_default() += 1;
        let mut resolved = Vec::with_capacity(node.deps.len());
        let mut failed = None;
        for dependency_key in &node.deps {
            let dependency = &self.graph.nodes[*dependency_key];
            match &dependency.status {
                NodeStatus::Done(Ok(output)) => resolved.push((dependency.descriptor.phase, dependency.cell, output.clone())),
                NodeStatus::Done(Err(failure)) => {
                    failed = Some(failure.clone());
                    break;
                }
                _ => unreachable!("dependency dispatched before it completed"),
            }
        }

        self.graph.nodes[key].status = NodeStatus::Running;
        #[cfg(feature = "debug")]
        {
            self.graph.nodes[key].started_at = Some(Instant::now());
        }

        // evicted before running, so a dependency nothing else holds can be moved out by take()
        for i in 0..self.graph.nodes[key].deps.len() {
            let dependency_key = self.graph.nodes[key].deps[i];
            self.try_evict(dependency_key);
        }

        if let Some(failure) = failed {
            let error = PhaseError::DependencyFailed(failure);
            return Err(Arc::new(PhaseFailure { phase: descriptor.name, cell, error }));
        }

        let inputs = PhaseInputs { resolved, generator: PhantomData };
        Ok((descriptor, cell, inputs))
    }

    pub fn execute(descriptor: PhaseDescriptor<G>, generator: &G, cell: ChunkPos, mut inputs: PhaseInputs<G>) -> Outcome {
        (descriptor.run)(generator, cell, &mut inputs).map_err(|error| Arc::new(PhaseFailure { phase: descriptor.name, cell, error }))
    }

    pub fn complete(&mut self, key: NodeKey, outcome: Outcome) {
        self.revision = self.revision.wrapping_add(1);
        let node = &mut self.graph.nodes[key];
        #[cfg(feature = "debug")]
        {
            node.duration = node.started_at.map(|started| started.elapsed());
        }
        if node.requested {
            node.requested = false;
            self.finished.push((node.cell, outcome.clone()));
        }
        let finished_terminal = (node.descriptor.phase == self.terminal).then_some(node.cell);
        node.status = NodeStatus::Done(outcome);
        if let Some(cell) = finished_terminal {
            self.completed.insert(cell);
        }

        for i in 0..self.graph.nodes[key].dependents.len() {
            let dependent_key = self.graph.nodes[key].dependents[i];
            let ready = match self.graph.nodes.get_mut(dependent_key).map(|dependent| &mut dependent.status) {
                Some(NodeStatus::Pending { remaining }) => {
                    *remaining -= 1;
                    *remaining == 0
                }
                _ => false,
            };
            if ready {
                self.push_ready(dependent_key);
            }
        }

        self.try_evict(key);
        if let Some(cell) = finished_terminal {
            self.sweep(cell);
        }
    }

    pub fn is_interior(&self, phase: PhaseId, cell: ChunkPos) -> bool {
        let Some(&reach) = self.reach.get(&phase) else { return false };
        let radius = reach.ceil() as i32;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if ((dx * dx + dz * dz) as f64).sqrt() > reach {
                    continue;
                }
                let around = ChunkPos::new(cell.x + dx, cell.z + dz);
                let finished = match self.graph.index.get(&(self.terminal, around, 0)) {
                    Some(&key) => matches!(self.graph.nodes[key].status, NodeStatus::Done(_)),
                    None => self.completed.contains(&around),
                };
                if !finished {
                    return false;
                }
            }
        }
        true
    }

    pub fn sweep(&mut self, around: ChunkPos) {
        let phases: Vec<PhaseId> = self.graph.retained.keys().copied().collect();
        for phase in phases {
            let Some(&reach) = self.reach.get(&phase) else { continue };
            let hops = self.hops.get(&phase).copied().unwrap_or(0);
            let radius = reach.ceil() as i32;
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    if ((dx * dx + dz * dz) as f64).sqrt() > reach {
                        continue;
                    }
                    let cell = ChunkPos::new(around.x + dx, around.z + dz);
                    for hop in 0..=hops {
                        let Some(&key) = self.graph.index.get(&(phase, cell, hop)) else { continue };
                        if self.graph.nodes[key].retained.is_some() && self.is_interior(phase, cell) {
                            self.remove(key);
                        }
                    }
                }
            }
        }
    }

    pub fn try_evict(&mut self, key: NodeKey) {
        let Some(node) = self.graph.nodes.get(key) else { return };
        if !matches!(node.status, NodeStatus::Done(_)) {
            return;
        }

        let still_needed = node
            .dependents
            .iter()
            .any(|&dependent| matches!(self.graph.nodes.get(dependent), Some(node) if matches!(node.status, NodeStatus::Pending { .. })));
        if still_needed {
            return;
        }

        let retain = node.descriptor.retain;
        if retain == 0 {
            self.remove(key);
            return;
        }
        if node.retained.is_some() {
            return;
        }
        if self.is_interior(node.descriptor.phase, node.cell) {
            self.remove(key);
            return;
        }

        let node = &mut self.graph.nodes[key];
        let phase = node.descriptor.phase;
        let name = node.descriptor.name;
        let retention = self.graph.retained.entry(phase).or_insert_with(|| Retention::new(name, retain));
        let stamp = retention.pushes;
        retention.pushes += 1;
        retention.queue.push_back((key, stamp));
        retention.stats.pushed += 1;
        node.retained = Some(stamp);
        node.dependents.clear();

        self.trim(phase);
    }

    pub fn trim(&mut self, phase: PhaseId) {
        let Some(retention) = self.graph.retained.get_mut(&phase) else { return };
        while retention.queue.len() > retention.capacity {
            let (oldest, oldest_stamp) = retention.queue.pop_front().expect("queue is longer than capacity");
            let Some(expired) = self.graph.nodes.get(oldest).filter(|node| node.retained == Some(oldest_stamp)) else {
                continue;
            };

            if expired.reused {
                retention.stats.evicted_after_reuse += 1;
            } else {
                retention.stats.evicted_unused += 1;
            }
            let ghost = (expired.cell, expired.hop);
            if retention.ghost_set.insert(ghost) {
                retention.ghosts.push_back(ghost);
            }
            if retention.ghosts.len() > retention_ceiling(retention.ceiling, self.focus.len()) {
                let forgotten = retention.ghosts.pop_front().expect("ghosts is longer than the ceiling");
                retention.ghost_set.remove(&forgotten);
            }

            let expired = &self.graph.nodes[oldest];
            self.graph.index.remove(&(expired.descriptor.phase, expired.cell, expired.hop));
            if let Some(node) = self.graph.nodes.remove(oldest)
                && let NodeStatus::Done(outcome) = node.status
            {
                self.discarded.push(outcome);
            }
        }
    }

    pub fn shrink_retention(&mut self) {
        let mut shrunk = Vec::new();
        for (&phase, retention) in &mut self.graph.retained {
            if retention.window_start.elapsed() < RETENTION_WINDOW {
                continue;
            }
            let depth = retention.window_max_depth as usize;
            let needed = depth + depth / 4;
            if retention.capacity > needed {
                let step = (retention.capacity / 8).max(1);
                retention.capacity = retention.capacity.saturating_sub(step).max(needed);
                shrunk.push(phase);
            }
            retention.window_start = Instant::now();
            retention.window_max_depth = 0;
        }
        for phase in shrunk {
            self.trim(phase);
        }
    }

    pub fn handle(&mut self, completion: Completion) {
        match completion {
            Completion::Finished(key, outcome) => {
                self.in_flight -= 1;
                self.complete(key, outcome);
            }
            Completion::Panicked(key) => {
                let node = &self.graph.nodes[key];
                panic!("{} at ({}, {}) panicked on a worker thread", node.descriptor.name, node.cell.x, node.cell.z);
            }
        }
    }

    pub fn fill_pool(&mut self, pool: &TaskPool) {
        while self.in_flight < pool.thread_num() * 2 {
            let Some(key) = self.pop_ready() else { break };
            let (descriptor, cell, inputs) = match self.start(key) {
                Ok(started) => started,
                Err(failure) => {
                    self.complete(key, Err(failure));
                    continue;
                }
            };
            let generator = self.generator.clone();
            let completions = self.completions_tx.clone();
            self.in_flight += 1;
            pool.spawn(async move {
                let report = PanicReport { key, completions };
                let outcome = Self::execute(descriptor, &generator, cell, inputs);
                let _ = report.completions.send(Completion::Finished(key, outcome));
            })
            .detach();
        }
    }

    pub fn remove(&mut self, key: NodeKey) {
        self.revision = self.revision.wrapping_add(1);
        let node = &self.graph.nodes[key];
        self.graph.index.remove(&(node.descriptor.phase, node.cell, node.hop));
        if let Some(node) = self.graph.nodes.remove(key)
            && let NodeStatus::Done(outcome) = node.status
            && node.descriptor.phase != self.terminal
        {
            self.discarded.push(outcome);
        }
    }

    pub fn release_one(&mut self) -> bool {
        self.discarded.pop();
        !self.discarded.is_empty()
    }

    pub fn cancel(&mut self, cell: ChunkPos) {
        let root = PhaseDescriptor::of::<G::Terminal>();
        let Some(&key) = self.graph.index.get(&(root.phase, cell, 0)) else { return };
        self.graph.nodes[key].requested = false;

        let mut unneeded = vec![key];
        while let Some(key) = unneeded.pop() {
            let Some(node) = self.graph.nodes.get(key) else { continue };
            if !matches!(node.status, NodeStatus::Pending { .. }) {
                self.try_evict(key);
                continue;
            }
            let still_needed = node.requested
                || node
                    .dependents
                    .iter()
                    .any(|&dependent| matches!(self.graph.nodes.get(dependent), Some(node) if matches!(node.status, NodeStatus::Pending { .. } | NodeStatus::Running)));
            if still_needed {
                continue;
            }

            unneeded.extend(node.deps.iter().copied());
            self.remove(key);
        }
    }

    pub fn pop_ready(&mut self) -> Option<NodeKey> {
        while let Some(Reverse((priority, key))) = self.pending_dispatch.pop() {
            if self
                .graph
                .nodes
                .get(key)
                .is_some_and(|node| node.priority == priority && matches!(node.status, NodeStatus::Pending { .. }))
            {
                return Some(key);
            }
        }
        None
    }
}

impl<G: Generator> WorldGenerator for PhaseGraph<G> {
    fn request_chunk(&mut self, x: i32, z: i32) {
        self.request(ChunkPos::new(x, z));
    }

    fn set_focus(&mut self, focus: &[(i32, i32)]) {
        PhaseGraph::set_focus(self, focus);
    }

    fn cancel_chunk(&mut self, x: i32, z: i32) {
        self.cancel(ChunkPos::new(x, z));
    }

    fn tick(&mut self) -> Vec<(i32, i32, Result<Chunk, Arc<PhaseFailure>>)> {
        while let Ok(completion) = self.completions_rx.try_recv() {
            self.handle(completion);
        }
        self.shrink_retention();
        if self.last_report.elapsed() >= REPORT_INTERVAL {
            self.last_report = Instant::now();
            tracing::debug!(target: "generation", "{}", self.progress_report());
        }

        let pool = AsyncComputeTaskPool::get();
        if pool.thread_num() == 0 {
            // one node per call, so the caller keeps control of how long it spends generating
            if let Some(key) = self.pop_ready() {
                let outcome = self.start(key).and_then(|(descriptor, cell, inputs)| Self::execute(descriptor, &self.generator, cell, inputs));
                self.complete(key, outcome);
            }
        } else {
            self.fill_pool(pool);

            // nothing more can be handed out until a worker finishes, so wait briefly instead of spinning
            if self.in_flight > 0
                && (self.in_flight >= pool.thread_num() * 2 || self.pending_dispatch.is_empty())
                && let Ok(completion) = self.completions_rx.recv_timeout(COMPLETION_WAIT)
            {
                self.handle(completion);
                while let Ok(completion) = self.completions_rx.try_recv() {
                    self.handle(completion);
                }
                self.fill_pool(pool);
            }
        }

        std::mem::take(&mut self.finished)
            .into_iter()
            .map(|(cell, outcome)| {
                let chunk = outcome.map(|output| {
                    let chunk = output.downcast::<Chunk>().expect("only the terminal phase is ever requested");
                    Arc::try_unwrap(chunk).unwrap_or_else(|shared| (*shared).clone())
                });
                (cell.x, cell.z, chunk)
            })
            .collect()
    }

    fn has_pending_work(&self) -> bool {
        !self.pending_dispatch.is_empty() || self.in_flight > 0
    }

    fn release(&mut self) -> bool {
        self.release_one()
    }

    fn retention_report(&self) -> String {
        let mut live: FxHashMap<PhaseId, usize> = FxHashMap::default();
        for node in self.graph.nodes.values() {
            if node.retained.is_some() {
                *live.entry(node.descriptor.phase).or_default() += 1;
            }
        }
        let mut lines: Vec<String> = self
            .graph
            .retained
            .iter()
            .map(|(phase, retention)| {
                let stats = retention.stats;
                let reused = 100.0 * stats.nodes_reused as f64 / stats.pushed.max(1) as f64;
                format!(
                    "{:<12} pushed {:>7} reused {:>7} ({:>5.1}%) reuse events {:>7} evicted unused {:>7} evicted after reuse {:>7} ghost hits {:>6} capacity {:>6} live retained {:>6}",
                    retention.name,
                    stats.pushed,
                    stats.nodes_reused,
                    reused,
                    stats.reuse_events,
                    stats.evicted_unused,
                    stats.evicted_after_reuse,
                    stats.ghost_hits,
                    retention.capacity,
                    live.get(phase).copied().unwrap_or(0)
                )
            })
            .collect();
        lines.sort();
        lines.join("\n")
    }

    fn stats(&self) -> GenerationStats {
        self.generation_stats()
    }
}

fn phase_shape<G: Generator>() -> (FxHashMap<PhaseId, f64>, FxHashMap<PhaseId, u32>) {
    let step = |offsets: &[(i32, i32)]| offsets.iter().map(|&(dx, dz)| ((dx * dx + dz * dz) as f64).sqrt()).fold(0.0, f64::max);
    let mut reach = FxHashMap::default();
    let mut hop_limits = FxHashMap::default();
    let mut stack = vec![(PhaseDescriptor::of::<G::Terminal>(), 0.0)];
    while let Some((descriptor, distance)) = stack.pop() {
        let requirements = (descriptor.requires)();
        let hops = requirements
            .iter()
            .filter(|requirement| requirement.descriptor.phase == descriptor.phase)
            .map(|requirement| step(requirement.offsets) * requirement.max_hops.unwrap_or(0) as f64)
            .fold(0.0, f64::max);
        let hop_limit = requirements
            .iter()
            .filter(|requirement| requirement.descriptor.phase == descriptor.phase)
            .map(|requirement| requirement.max_hops.unwrap_or(0))
            .max()
            .unwrap_or(0);
        hop_limits.insert(descriptor.phase, hop_limit);
        let distance = distance + hops;
        if reach.get(&descriptor.phase).is_some_and(|&known| known >= distance) {
            continue;
        }
        reach.insert(descriptor.phase, distance);
        for requirement in requirements.iter().filter(|requirement| requirement.descriptor.phase != descriptor.phase) {
            stack.push((requirement.descriptor, distance + step(requirement.offsets)));
        }
    }
    (reach, hop_limits)
}

impl<G: Generator> PhaseGraph<G> {
    fn generation_stats(&self) -> GenerationStats {
        let mut live: FxHashMap<&'static str, usize> = FxHashMap::default();
        let mut waiting = 0;
        for node in self.graph.nodes.values() {
            if node.retained.is_some() {
                *live.entry(node.descriptor.name).or_default() += 1;
            }
            if matches!(node.status, NodeStatus::Done(_)) && node.retained.is_none() {
                waiting += 1;
            }
        }
        let mut retained: Vec<(&'static str, usize)> = live.into_iter().collect();
        retained.sort_unstable();
        let mut started: Vec<(&'static str, u64)> = self.started.iter().map(|(&name, &count)| (name, count)).collect();
        started.sort_unstable();
        GenerationStats {
            nodes: self.graph.nodes.len(),
            waiting,
            discarded: self.discarded.len(),
            completed: self.completed.len(),
            retained,
            started,
        }
    }

    fn progress_report(&self) -> String {
        let stats = self.generation_stats();
        let list = |items: Vec<String>| items.join(" ");
        format!(
            "nodes {} done-unretained {} discarded {} retained [{}] chunks completed {} started {}",
            stats.nodes,
            stats.waiting,
            stats.discarded,
            list(stats.retained.iter().map(|(name, count)| format!("{name}={count}")).collect()),
            stats.completed,
            list(stats.started.iter().map(|(name, count)| format!("{name}={count}")).collect())
        )
    }
}

fn retention_ceiling(per_focus: usize, foci: usize) -> usize {
    per_focus.saturating_mul(foci.max(1))
}

fn nearest_distance(by_x: &[ChunkPos], cell: ChunkPos) -> Option<f64> {
    let split = by_x.partition_point(|focus| focus.x < cell.x);
    let mut best: Option<f64> = None;
    let mut visit = |focus: &ChunkPos| {
        let dx = (focus.x - cell.x) as f64;
        if best.is_some_and(|best| dx.abs() >= best) {
            return false;
        }
        let dz = (cell.z - focus.z) as f64;
        let distance = (dx * dx + dz * dz).sqrt();
        best = Some(best.map_or(distance, |best| best.min(distance)));
        true
    };
    by_x[split..].iter().all(&mut visit);
    by_x[..split].iter().rev().all(&mut visit);
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute_force(foci: &[ChunkPos], cell: ChunkPos) -> Option<f64> {
        foci.iter()
            .map(|focus| (((cell.x - focus.x) as f64).powi(2) + ((cell.z - focus.z) as f64).powi(2)).sqrt())
            .reduce(f64::min)
    }

    #[test]
    fn the_retention_ceiling_scales_with_the_number_of_foci() {
        assert_eq!(retention_ceiling(1024, 0), 1024);
        assert_eq!(retention_ceiling(1024, 1), 1024);
        assert_eq!(retention_ceiling(1024, 100), 102_400);
        assert_eq!(retention_ceiling(usize::MAX, 4), usize::MAX);
    }

    #[test]
    fn no_foci_have_no_distance() {
        assert_eq!(nearest_distance(&[], ChunkPos::new(3, 4)), None);
    }

    #[test]
    fn the_nearest_focus_matches_a_scan_of_every_focus() {
        let mut state = 0x2545f4914f6cdd1du64;
        let mut next = |span: i32| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % span as u64) as i32 - span / 2
        };
        for count in [1usize, 2, 5, 40, 150] {
            let mut foci: Vec<ChunkPos> = (0..count).map(|_| ChunkPos::new(next(600), next(600))).collect();
            foci.sort_unstable_by_key(|focus| focus.x);
            for _ in 0..500 {
                let cell = ChunkPos::new(next(800), next(800));
                assert_eq!(nearest_distance(&foci, cell), brute_force(&foci, cell), "{count} foci, cell {cell:?}");
            }
        }
    }
}
