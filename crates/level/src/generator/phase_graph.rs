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
use crate::generator::footprint::{self, Footprints, Region, footprints};
use crate::generator::graph::{Graph, Node, NodeKey, NodeMail, NodeStatus, Outcome, Retention};
use crate::generator::phase::{InboxEntry, Outbox, PhaseDescriptor, PhaseId, PhaseInputs};
use crate::generator::pos::ChunkPos;

/// Sweep the retained nodes once their number has grown by this factor since the last sweep.
const REPORT_INTERVAL: Duration = Duration::from_secs(30);
const COMPLETION_WAIT: Duration = Duration::from_millis(1);

pub enum Completion {
    Finished(NodeKey, Outcome, Outbox),
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
    pub radius: i32,
    pub retention_budget: f64,
    pub lookahead: i32,
    pub abandoned: FxHashSet<NodeKey>,
    pub mail_reads: FxHashMap<PhaseId, Vec<MailRead>>,
    pub mail_consumers: FxHashMap<PhaseId, Vec<MailConsumer>>,
    pub reach: FxHashMap<PhaseId, f64>,
    pub hops: FxHashMap<PhaseId, u32>,
    pub footprints: Footprints,
    pub regions: FxHashMap<(PhaseId, u32), Region>,
    pub extent: f64,
    pub terminal: PhaseId,
    pub completed: FxHashSet<ChunkPos>,
    pub started: FxHashMap<&'static str, u64>,
    pub last_report: Instant,
    pub in_flight: usize,
    pub finished: Vec<(ChunkPos, Outcome)>,
    pub discarded: Vec<Outcome>,
    pub to_release: Vec<NodeKey>,
    pub revision: u64,
}

impl<G: Generator> PhaseGraph<G> {
    pub fn new(generator: G) -> Self {
        let (completions_tx, completions_rx) = crossbeam_channel::unbounded();
        let (reach, hops) = phase_shape::<G>();
        let footprints = footprints::<G>();
        let extent = footprint::extent(&footprints);
        let regions = footprints.iter().map(|(&key, cells)| (key, Region::new(cells, 0))).collect();
        let (mail_reads, mail_consumers) = phase_mail::<G>();

        Self {
            generator: Arc::new(generator),
            graph: Graph::new(),
            completions_tx,
            completions_rx,
            pending_dispatch: BinaryHeap::new(),
            next_order: 0,
            focus: Vec::new(),
            focus_by_x: Vec::new(),
            radius: 0,
            retention_budget: 1.0,
            lookahead: 0,
            abandoned: FxHashSet::default(),
            mail_reads,
            mail_consumers,
            reach,
            hops,
            footprints,
            regions,
            extent,
            terminal: PhaseId::of::<G::Terminal>(),
            completed: FxHashSet::default(),
            started: FxHashMap::default(),
            last_report: Instant::now(),
            in_flight: 0,
            finished: Vec::new(),
            discarded: Vec::new(),
            to_release: Vec::new(),
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
                _ => {
                    node.requested = true;
                    self.abandoned.remove(&key);
                }
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
                reader: Option<(ChunkPos, &'static [(i32, i32)])>,
            },
            Finalize {
                key: NodeKey,
                dep_keys: Vec<(PhaseId, ChunkPos, u32)>,
                invalid: Option<PhaseError>,
            },
        }

        let mut work = vec![Step::Enter {
            descriptor,
            cell,
            hop: 0,
            reader: None,
        }];

        while let Some(step) = work.pop() {
            match step {
                Step::Enter { descriptor, cell, hop, reader } => {
                    let map_key = (descriptor.phase, cell, hop);
                    if let Some(&existing) = self.graph.index.get(&map_key) {
                        if !self.lacks_mail_for(existing, reader) {
                            continue;
                        }
                        self.close(existing);
                    }

                    let phase = self.graph.phases.slot(descriptor);
                    let key = self.graph.nodes.insert(Node {
                        reused: false,
                        phase,
                        generator: PhantomData,
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
                                reader: requirement.mail_for.map(|targets| (cell, targets)),
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
                            phase: self.graph.phases.name(node),
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
        let reach = self.reach.get(&self.graph.phases.id(node)).copied().unwrap_or(0.0);
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

    pub fn set_focus(&mut self, focus: &[(i32, i32)], radius: i32) {
        let focus: Vec<ChunkPos> = focus.iter().map(|&(x, z)| ChunkPos::new(x, z)).collect();
        if focus == self.focus && radius == self.radius {
            return;
        }
        let jumped = focus.len() != self.focus.len() || radius < self.radius || focus.iter().zip(&self.focus).any(|(now, before)| now.x.abs_diff(before.x).max(now.z.abs_diff(before.z)) > 1);
        let steps: Vec<(ChunkPos, ChunkPos)> = self.focus.iter().copied().zip(focus.iter().copied()).filter(|(before, now)| before != now).collect();
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
        let descriptor = self.graph.phases.descriptor(node);
        let cell = node.cell;
        let hop = node.hop;
        *self.started.entry(descriptor.name).or_default() += 1;
        let mut resolved = Vec::with_capacity(node.deps.len());
        let mut failed = None;
        for dependency_key in &node.deps {
            let dependency = &self.graph.nodes[*dependency_key];
            match &dependency.status {
                NodeStatus::Done(Ok(output)) => resolved.push((self.graph.phases.id(dependency), dependency.cell, output.clone())),
                NodeStatus::Done(Err(failure)) => {
                    failed = Some(failure.clone());
                    break;
                }
                _ => unreachable!("dependency dispatched before it completed"),
            }
        }

        let inbox = self.collect_mail(descriptor.phase, cell, hop);

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

        let inputs = PhaseInputs {
            resolved,
            generator: PhantomData,
            phase: descriptor.phase,
            inbox,
            outbox: Vec::new(),
        };
        Ok((descriptor, cell, inputs))
    }

    pub fn execute(descriptor: PhaseDescriptor<G>, generator: &G, cell: ChunkPos, mut inputs: PhaseInputs<G>) -> (Outcome, Outbox) {
        let outcome = (descriptor.run)(generator, cell, &mut inputs).map_err(|error| Arc::new(PhaseFailure { phase: descriptor.name, cell, error }));
        (outcome, std::mem::take(&mut inputs.outbox))
    }

    pub fn complete(&mut self, key: NodeKey, outcome: Outcome) {
        self.revision = self.revision.wrapping_add(1);
        self.abandoned.remove(&key);
        let node = &mut self.graph.nodes[key];
        #[cfg(feature = "debug")]
        {
            node.duration = node.started_at.map(|started| started.elapsed());
        }
        if node.requested {
            node.requested = false;
            self.finished.push((node.cell, outcome.clone()));
        }
        let finished_terminal = (self.graph.phases.id(node) == self.terminal).then_some(node.cell);
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

        self.release_mail_read_by(key);
        self.try_evict(key);
        if let Some(cell) = finished_terminal {
            self.sweep(cell);
        }
    }

    fn release_mail_read_by(&mut self, key: NodeKey) {
        let node = &self.graph.nodes[key];
        let (phase, cell, hop) = (self.graph.phases.id(node), node.cell, node.hop);
        let Some(reads) = self.mail_reads.get(&phase) else { return };
        let mut producers = Vec::new();
        for read in reads {
            let producer_hop = match (read.producer == phase, read.max_hops) {
                (false, _) => 0,
                (true, Some(max_hops)) if hop < max_hops => hop + 1,
                _ => continue,
            };
            for &(dx, dz) in read.offsets {
                let source = ChunkPos::new(cell.x + dx, cell.z + dz);
                if let Some(&producer) = self.graph.index.get(&(read.producer, source, producer_hop)) {
                    producers.push(producer);
                }
            }
        }
        for producer in producers {
            self.prune_mail(producer);
        }
    }

    pub fn is_interior(&self, phase: PhaseId, hop: u32, cell: ChunkPos) -> bool {
        let Some(footprint) = self.footprints.get(&(phase, hop)) else { return false };
        footprint.iter().all(|&(dx, dz)| self.cell_finished(ChunkPos::new(cell.x + dx, cell.z + dz)))
    }

    pub fn sweep(&mut self, around: ChunkPos) {
        let phases: Vec<PhaseId> = self.graph.retained.keys().copied().collect();
        for phase in phases {
            let hops = self.hops.get(&phase).copied().unwrap_or(0);
            for hop in 0..=hops {
                let Some(footprint) = self.footprints.get(&(phase, hop)) else { continue };
                let interior: Vec<NodeKey> = footprint
                    .iter()
                    .filter_map(|&(dx, dz)| {
                        let cell = ChunkPos::new(around.x - dx, around.z - dz);
                        let &key = self.graph.index.get(&(phase, cell, hop))?;
                        (self.graph.nodes[key].retained.is_some() && self.is_interior(phase, hop, cell)).then_some(key)
                    })
                    .collect();
                for key in interior {
                    self.remove(key);
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

        let retain = self.graph.phases.retain(node);
        if retain == 0 {
            self.remove(key);
            return;
        }
        if node.retained.is_some() {
            return;
        }
        if self.is_interior(self.graph.phases.id(node), node.hop, node.cell) || !self.in_region(self.graph.phases.id(node), node.hop, node.cell) {
            self.remove(key);
            return;
        }

        let node = &mut self.graph.nodes[key];
        let phase = self.graph.phases.id(node);
        let name = self.graph.phases.name(node);
        let retention = self.graph.retained.entry(phase).or_insert_with(|| Retention::new(name));
        let stamp = retention.pushes;
        retention.pushes += 1;
        retention.entries.insert(stamp, key);
        retention.stats.pushed += 1;
        node.retained = Some(stamp);
        node.dependents.clear();
        self.prune_mail(key);
        self.enforce_cap(phase);
    }

    fn collect_mail(&self, phase: PhaseId, cell: ChunkPos, hop: u32) -> Vec<InboxEntry> {
        let mut inbox = Vec::new();
        let Some(reads) = self.mail_reads.get(&phase) else { return inbox };
        for read in reads {
            let dependency_hop = match (read.producer == phase, read.max_hops) {
                (false, _) => 0,
                (true, Some(max_hops)) if hop < max_hops => hop + 1,
                _ => continue,
            };
            for &(dx, dz) in read.offsets {
                let source = ChunkPos::new(cell.x + dx, cell.z + dz);
                let Some(&producer) = self.graph.index.get(&(read.producer, source, dependency_hop)) else {
                    continue;
                };
                let Some(mail) = self.graph.mail.get(&producer) else { continue };
                for (target, value) in &mail.out {
                    if read.targets.iter().any(|&(tx, tz)| ChunkPos::new(cell.x + tx, cell.z + tz) == *target) {
                        inbox.push(InboxEntry {
                            producer: read.producer,
                            target: *target,
                            source,
                            value: value.clone(),
                        });
                    }
                }
            }
        }
        inbox
    }

    fn store_mail(&mut self, key: NodeKey, mail: Outbox) {
        if mail.is_empty() || !self.graph.nodes.contains_key(key) {
            return;
        }
        self.graph.mail.insert(key, NodeMail { out: mail, pruned: Vec::new() });
    }

    /// Whether the cell `reader` asks about could still be consumed by nothing but a node that has had mail pruned for it.
    fn lacks_mail_for(&self, key: NodeKey, reader: Option<(ChunkPos, &'static [(i32, i32)])>) -> bool {
        let Some((consumer, targets)) = reader else { return false };
        let Some(mail) = self.graph.mail.get(&key) else { return false };
        mail.pruned
            .iter()
            .any(|pruned| targets.iter().any(|&(dx, dz)| ChunkPos::new(consumer.x + dx, consumer.z + dz) == *pruned))
    }

    /// Takes a node out of the index so the next request builds a fresh one. Whatever already depends on it still finishes with it.
    fn close(&mut self, key: NodeKey) {
        if self.graph.nodes[key].retained.is_some() {
            self.evict_retained(key);
        } else {
            self.unindex(key);
        }
    }

    fn unindex(&mut self, key: NodeKey) {
        let Some(node) = self.graph.nodes.get(key) else { return };
        let id = (self.graph.phases.id(node), node.cell, node.hop);
        if self.graph.index.get(&id) == Some(&key) {
            self.graph.index.remove(&id);
        }
    }

    /// Whether some phase could still read what the node at `cell` and `hop` addressed to `target`. A reader is a cell that lists this node among its
    /// sources and `target` among its targets. It is done reading when its node has run, and it cannot read later if the cell is finished or out of demand.
    fn mail_alive(&self, phase: PhaseId, cell: ChunkPos, hop: u32, target: ChunkPos) -> bool {
        let Some(consumers) = self.mail_consumers.get(&phase) else { return true };
        for consumer in consumers {
            let hops = if consumer.phase == phase {
                match (hop.checked_sub(1), consumer.max_hops) {
                    (Some(reader_hop), Some(max_hops)) if reader_hop < max_hops => reader_hop..=reader_hop,
                    _ => continue,
                }
            } else {
                0..=self.hops.get(&consumer.phase).copied().unwrap_or(0)
            };
            for &(ox, oz) in consumer.offsets {
                let reader = ChunkPos::new(cell.x - ox, cell.z - oz);
                if !consumer.targets.iter().any(|&(tx, tz)| ChunkPos::new(reader.x + tx, reader.z + tz) == target) {
                    continue;
                }
                for reader_hop in hops.clone() {
                    match self.graph.index.get(&(consumer.phase, reader, reader_hop)).map(|&key| &self.graph.nodes[key].status) {
                        Some(NodeStatus::Done(_)) => {}
                        Some(_) => return true,
                        None => {
                            if !self.cell_finished(reader) && self.in_region(consumer.phase, reader_hop, reader) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn cell_finished(&self, cell: ChunkPos) -> bool {
        match self.graph.index.get(&(self.terminal, cell, 0)) {
            Some(&key) => matches!(self.graph.nodes[key].status, NodeStatus::Done(_)),
            None => self.completed.contains(&cell),
        }
    }

    fn prune_mail(&mut self, key: NodeKey) {
        let Some(node) = self.graph.nodes.get(key) else { return };
        let Some(mail) = self.graph.mail.get(&key) else { return };
        if mail.out.is_empty() {
            return;
        }
        let (phase, cell, hop) = (self.graph.phases.id(node), node.cell, node.hop);
        let keep: Vec<bool> = mail.out.iter().map(|(target, _)| self.mail_alive(phase, cell, hop, *target)).collect();
        if keep.iter().all(|&kept| kept) {
            return;
        }
        let mail = self.graph.mail.get_mut(&key).expect("mail was just read");
        let mut keep = keep.into_iter();
        let mut dropped = Vec::new();
        mail.out.retain(|(target, _)| {
            let kept = keep.next().unwrap_or(true);
            if !kept {
                dropped.push(*target);
            }
            kept
        });
        mail.pruned.extend(dropped);
    }

    /// The most nodes of one phase to keep: the budget times the chunks in view, for every focus.
    fn retention_cap(&self) -> usize {
        let view_chunks = std::f64::consts::PI * (self.radius as f64).powi(2);
        (self.retention_budget * view_chunks) as usize * self.focus.len()
    }

    fn enforce_cap(&mut self, phase: PhaseId) {
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

    fn rebuild_regions(&mut self) {
        let demand = self.radius + self.lookahead;
        self.regions = self.footprints.iter().map(|(&key, cells)| (key, Region::new(cells, demand))).collect();
    }

    pub fn in_region(&self, phase: PhaseId, hop: u32, cell: ChunkPos) -> bool {
        if self.focus.is_empty() {
            return true;
        }
        let Some(region) = self.regions.get(&(phase, hop)) else { return false };
        let from = self.focus_by_x.partition_point(|focus| focus.x < cell.x - region.half());
        self.focus_by_x[from..]
            .iter()
            .take_while(|focus| focus.x <= cell.x + region.half())
            .any(|focus| region.contains(focus.x - cell.x, focus.z - cell.z))
    }

    pub fn sweep_retained(&mut self) {
        self.abandon_unwanted();
        let outside: Vec<NodeKey> = self
            .graph
            .retained
            .values()
            .flat_map(|retention| retention.entries.values().copied())
            .filter(|&key| self.graph.nodes.get(key).is_none_or(|node| !self.in_region(self.graph.phases.id(node), node.hop, node.cell)))
            .collect();
        for key in outside {
            self.evict_retained(key);
        }
        let remaining: Vec<NodeKey> = self.graph.retained.values().flat_map(|retention| retention.entries.values().copied()).collect();
        for key in remaining {
            self.prune_mail(key);
        }
    }

    /// The cells whose nodes of `phase` at `hop` were in the region of a focus at `from` and are not in the region of the same focus at `to`.
    fn cells_left_behind(&self, phase: PhaseId, hop: u32, from: ChunkPos, to: ChunkPos) -> Vec<ChunkPos> {
        let Some(region) = self.regions.get(&(phase, hop)) else { return Vec::new() };
        let half = region.half();
        let (dx, dz) = (to.x - from.x, to.z - from.z);
        let mut left = Vec::new();
        for ox in -half..=half {
            for oz in -half..=half {
                if region.contains(ox, oz) && !region.contains(ox + dx, oz + dz) {
                    left.push(ChunkPos::new(from.x - ox, from.z - oz));
                }
            }
        }
        left
    }

    /// Frees what a focus stepping from `from` to `to` left behind, looking only at the cells it stopped covering.
    fn release_left_behind(&mut self, from: ChunkPos, to: ChunkPos) {
        let retained_phases: Vec<PhaseId> = self.graph.retained.keys().copied().collect();
        for phase in retained_phases {
            let hops = self.hops.get(&phase).copied().unwrap_or(0);
            for hop in 0..=hops {
                let outside: Vec<NodeKey> = self
                    .cells_left_behind(phase, hop, from, to)
                    .into_iter()
                    .filter(|&cell| !self.in_region(phase, hop, cell))
                    .filter_map(|cell| self.graph.index.get(&(phase, cell, hop)).copied())
                    .filter(|&key| self.graph.nodes[key].retained.is_some())
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
                            let cell = ChunkPos::new(reader.x + ox, reader.z + oz);
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
            if self.graph.nodes[key].retained.is_some() {
                self.prune_mail(key);
            }
        }
    }

    fn evict_retained(&mut self, key: NodeKey) {
        let Some(node) = self.graph.nodes.get_mut(key) else { return };
        let stamp = node.retained.take();
        let (phase, reused) = (self.graph.phases.id(node), node.reused);
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

    pub fn handle(&mut self, completion: Completion) {
        match completion {
            Completion::Finished(key, outcome, pushes) => {
                self.in_flight -= 1;

                if outcome.is_ok() {
                    self.store_mail(key, pushes);
                }
                self.complete(key, outcome);
            }
            Completion::Panicked(key) => {
                let node = &self.graph.nodes[key];
                panic!("{} at ({}, {}) panicked on a worker thread", self.graph.phases.name(node), node.cell.x, node.cell.z);
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
                let (outcome, pushes) = Self::execute(descriptor, &generator, cell, inputs);
                let _ = report.completions.send(Completion::Finished(key, outcome, pushes));
            })
            .detach();
        }
    }

    pub fn remove(&mut self, key: NodeKey) {
        self.revision = self.revision.wrapping_add(1);
        self.abandoned.remove(&key);
        let node = &self.graph.nodes[key];
        if let Some(stamp) = node.retained
            && let Some(retention) = self.graph.retained.get_mut(&self.graph.phases.id(node))
        {
            retention.entries.remove(&stamp);
        }
        self.unindex(key);
        if let Some(node) = self.graph.remove_node(key)
            && let NodeStatus::Done(outcome) = node.status
            && self.graph.phases.id_at(node.phase) != self.terminal
        {
            self.discarded.push(outcome);
        }
    }

    pub fn release_one(&mut self) -> bool {
        if let Some(key) = self.to_release.pop() {
            if let Some(node) = self.graph.remove_node(key)
                && let NodeStatus::Done(outcome) = node.status
            {
                self.discarded.push(outcome);
            }
            return !self.to_release.is_empty() || !self.discarded.is_empty();
        }
        self.discarded.pop();
        !self.discarded.is_empty()
    }

    /// Drops the request for a chunk. A pipeline that is still running is not torn down: a request that comes back reuses it,
    /// and it is released only once the chunk is out of every focus's demand region.
    pub fn cancel(&mut self, cell: ChunkPos) {
        let root = PhaseDescriptor::of::<G::Terminal>();
        let Some(&key) = self.graph.index.get(&(root.phase, cell, 0)) else { return };
        let node = &mut self.graph.nodes[key];
        node.requested = false;
        if matches!(node.status, NodeStatus::Done(_)) {
            self.try_evict(key);
        } else {
            self.abandoned.insert(key);
        }
    }

    /// Releases the pipelines of chunks nobody asks for any more and that no focus can reach: further out than the view plus
    /// the longest dependency chain, so nothing a requested chunk could need depends on them.
    pub fn abandon_unwanted(&mut self) {
        let furthest = self.extent + (self.radius + self.lookahead) as f64;
        let doomed: Vec<NodeKey> = self
            .abandoned
            .iter()
            .copied()
            .filter(|&key| {
                self.graph
                    .nodes
                    .get(key)
                    .is_none_or(|node| node.requested || self.nearest_focus(node.cell).is_none_or(|distance| distance > furthest))
            })
            .collect();
        for key in doomed {
            self.abandoned.remove(&key);
            if self.graph.nodes.get(key).is_some_and(|node| !node.requested) {
                self.release_pipeline(key);
            }
        }
    }

    fn release_pipeline(&mut self, key: NodeKey) {
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

    fn set_retention_budget(&mut self, budget: f64) {
        self.retention_budget = budget.max(0.0);
    }

    fn set_lookahead(&mut self, lookahead: i32) {
        let shrank = lookahead.max(0) < self.lookahead;
        self.lookahead = lookahead.max(0);
        self.rebuild_regions();
        if shrank {
            self.sweep_retained();
        }
    }

    fn set_focus(&mut self, focus: &[(i32, i32)], radius: i32) {
        PhaseGraph::set_focus(self, focus, radius);
    }

    fn cancel_chunk(&mut self, x: i32, z: i32) {
        self.cancel(ChunkPos::new(x, z));
    }

    fn tick(&mut self) -> Vec<(i32, i32, Result<Chunk, Arc<PhaseFailure>>)> {
        while let Ok(completion) = self.completions_rx.try_recv() {
            self.handle(completion);
        }
        if self.last_report.elapsed() >= REPORT_INTERVAL {
            self.last_report = Instant::now();
            tracing::debug!(target: "generation", "{}", self.progress_report());
        }

        let pool = AsyncComputeTaskPool::get();
        if pool.thread_num() == 0 {
            // one node per call, so the caller keeps control of how long it spends generating
            if let Some(key) = self.pop_ready() {
                let result = self.start(key).and_then(|(descriptor, cell, inputs)| {
                    let (outcome, pushes) = Self::execute(descriptor, &self.generator, cell, inputs);
                    if outcome.is_ok() {
                        self.store_mail(key, pushes);
                    }
                    outcome
                });
                self.complete(key, result);
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
                *live.entry(self.graph.phases.id(node)).or_default() += 1;
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
                    "{:<12} pushed {:>7} reused {:>7} ({:>5.1}%) reuse events {:>7} evicted unused {:>7} evicted after reuse {:>7} live retained {:>6}",
                    retention.name,
                    stats.pushed,
                    stats.nodes_reused,
                    reused,
                    stats.reuse_events,
                    stats.evicted_unused,
                    stats.evicted_after_reuse,
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

    #[cfg(feature = "debug")]
    fn debug_graph(&self) -> Option<&dyn crate::generator::debug::DebugGraph> {
        Some(self)
    }
}

#[cfg(feature = "debug")]
impl<G: Generator> crate::generator::debug::DebugGraph for PhaseGraph<G> {
    fn terminal(&self) -> PhaseId {
        self.terminal
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn focus(&self) -> &[ChunkPos] {
        &self.focus
    }

    fn completed(&self) -> &FxHashSet<ChunkPos> {
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
        use crate::generator::debug::{DebugPhase, DebugRequirement};

        let mut phases = Vec::new();
        let mut seen = FxHashSet::default();
        let mut stack = vec![PhaseDescriptor::of::<G::Terminal>()];
        while let Some(descriptor) = stack.pop() {
            if !seen.insert(descriptor.phase) {
                continue;
            }
            let requirements = (descriptor.requires)()
                .into_iter()
                .map(|requirement| {
                    if requirement.descriptor.phase != descriptor.phase {
                        stack.push(requirement.descriptor);
                    }
                    DebugRequirement {
                        phase: requirement.descriptor.phase,
                        max_hops: requirement.max_hops,
                    }
                })
                .collect();
            phases.push(DebugPhase {
                id: descriptor.phase,
                name: descriptor.name,
                retain: descriptor.retain,
                requirements,
            });
        }
        phases
    }

    fn node(&self, phase: PhaseId, cell: ChunkPos, hop: u32) -> Option<crate::generator::debug::DebugNode> {
        self.graph.index.get(&(phase, cell, hop)).and_then(|&key| {
            self.graph.nodes.get(key).map(|node| crate::generator::debug::DebugNode {
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
                    .filter_map(|&dep| self.graph.nodes.get(dep))
                    .map(|dep| (self.graph.phases.id(dep), dep.cell, dep.hop))
                    .collect(),
            })
        })
    }

    fn for_each_node(&self, f: &mut dyn FnMut(crate::generator::debug::DebugNode)) {
        for node in self.graph.nodes.values() {
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
                    .filter_map(|&dep| self.graph.nodes.get(dep))
                    .map(|dep| (self.graph.phases.id(dep), dep.cell, dep.hop))
                    .collect(),
            });
        }
    }

    fn nodes_at(&self, cell: ChunkPos) -> Vec<crate::generator::debug::DebugNode> {
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

fn offsets_step(offsets: &[(i32, i32)]) -> f64 {
    offsets.iter().map(|&(dx, dz)| ((dx * dx + dz * dz) as f64).sqrt()).fold(0.0, f64::max)
}

fn phase_shape<G: Generator>() -> (FxHashMap<PhaseId, f64>, FxHashMap<PhaseId, u32>) {
    let mut reach = FxHashMap::default();
    let mut hop_limits = FxHashMap::default();
    let mut stack = vec![(PhaseDescriptor::of::<G::Terminal>(), 0.0)];
    while let Some((descriptor, distance)) = stack.pop() {
        let requirements = (descriptor.requires)();
        let hops = requirements
            .iter()
            .filter(|requirement| requirement.descriptor.phase == descriptor.phase)
            .map(|requirement| offsets_step(requirement.offsets) * requirement.max_hops.unwrap_or(0) as f64)
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
            stack.push((requirement.descriptor, distance + offsets_step(requirement.offsets)));
        }
    }
    (reach, hop_limits)
}

/// One mail declaration of a phase: it reads what the producer cells at `offsets` addressed to the cells at `targets`.
pub struct MailRead {
    pub producer: PhaseId,
    pub offsets: &'static [(i32, i32)],
    pub targets: &'static [(i32, i32)],
    pub max_hops: Option<u32>,
}

/// A phase that reads some producer's mail, and which cells of the producer it reads from and for.
pub struct MailConsumer {
    pub phase: PhaseId,
    pub offsets: &'static [(i32, i32)],
    pub targets: &'static [(i32, i32)],
    pub max_hops: Option<u32>,
}

fn phase_mail<G: Generator>() -> (FxHashMap<PhaseId, Vec<MailRead>>, FxHashMap<PhaseId, Vec<MailConsumer>>) {
    let mut reads: FxHashMap<PhaseId, Vec<MailRead>> = FxHashMap::default();
    let mut consumers: FxHashMap<PhaseId, Vec<MailConsumer>> = FxHashMap::default();
    let mut seen = FxHashSet::default();
    let mut stack = vec![PhaseDescriptor::of::<G::Terminal>()];
    while let Some(descriptor) = stack.pop() {
        if !seen.insert(descriptor.phase) {
            continue;
        }
        for requirement in (descriptor.requires)() {
            if requirement.descriptor.phase != descriptor.phase {
                stack.push(requirement.descriptor);
            }
            let Some(targets) = requirement.mail_for else { continue };
            reads.entry(descriptor.phase).or_default().push(MailRead {
                producer: requirement.descriptor.phase,
                offsets: requirement.offsets,
                targets,
                max_hops: requirement.max_hops,
            });
            consumers.entry(requirement.descriptor.phase).or_default().push(MailConsumer {
                phase: descriptor.phase,
                offsets: requirement.offsets,
                targets,
                max_hops: requirement.max_hops,
            });
        }
    }
    (reads, consumers)
}

impl<G: Generator> PhaseGraph<G> {
    fn generation_stats(&self) -> GenerationStats {
        let mut live: FxHashMap<&'static str, usize> = FxHashMap::default();
        let mut waiting = 0;
        for node in self.graph.nodes.values() {
            if node.retained.is_some() {
                *live.entry(self.graph.phases.name(node)).or_default() += 1;
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

    mod mail {
        use super::*;
        use crate::generator::phase::{Mailing, Phase, SAME_CELL};
        use std::sync::Mutex;

        struct Toy;

        impl Generator for Toy {
            type Terminal = SinkPhase;
        }

        struct SourcePhase;

        impl Phase<Toy> for SourcePhase {
            type Output = ();

            const RETAIN: usize = 16;

            fn run(_: &Toy, cell: ChunkPos, inputs: &mut PhaseInputs<Toy>) -> Result<(), PhaseError> {
                SOURCE_RUNS.lock().unwrap().push(cell);
                inputs.send::<SourcePhase>(cell, 100 + cell.x);
                inputs.send::<SourcePhase>(ChunkPos::new(cell.x + 1, cell.z), 200 + cell.x);
                inputs.send::<SourcePhase>(ChunkPos::new(cell.x + 50, cell.z), 999);
                Ok(())
            }
        }

        impl Mailing<Toy> for SourcePhase {
            type Mail = i32;
        }

        const READ_FROM: &[(i32, i32)] = &[(-1, 0), (0, 0), (1, 0)];
        static SEEN: Mutex<Vec<(ChunkPos, Vec<(ChunkPos, i32)>)>> = Mutex::new(Vec::new());
        static SOURCE_RUNS: Mutex<Vec<ChunkPos>> = Mutex::new(Vec::new());

        struct SinkPhase;

        impl Phase<Toy> for SinkPhase {
            type Output = Chunk;

            fn requires() -> Vec<crate::generator::phase::Requirement<Toy>> {
                vec![SourcePhase::at_with_mail(READ_FROM, SAME_CELL)]
            }

            fn run(_: &Toy, cell: ChunkPos, inputs: &mut PhaseInputs<Toy>) -> Result<Chunk, PhaseError> {
                let received = inputs.mail::<SourcePhase>(cell).map(|(source, mail)| (source, *mail)).collect();
                SEEN.lock().unwrap().push((cell, received));
                Ok(Chunk::new(cell.x, cell.z, -4, 24, 0, 1))
            }
        }

        fn finish(graph: &mut PhaseGraph<Toy>, cells: &[ChunkPos]) {
            AsyncComputeTaskPool::get_or_init(TaskPool::default);
            for &cell in cells {
                graph.request(cell);
            }
            let mut done = 0;
            let start = Instant::now();
            while done < cells.len() {
                done += WorldGenerator::tick(graph).len();
                assert!(start.elapsed() < Duration::from_secs(30), "generation never finished");
            }
        }

        #[test]
        fn a_phase_receives_only_the_mail_it_declared_in_the_order_it_declared_it() {
            let mut graph = PhaseGraph::new(Toy);
            finish(&mut graph, &[ChunkPos::new(5, 0)]);
            let seen = SEEN.lock().unwrap();
            let (_, received) = seen.iter().find(|(cell, _)| *cell == ChunkPos::new(5, 0)).expect("the sink ran");
            assert_eq!(received, &[(ChunkPos::new(4, 0), 204), (ChunkPos::new(5, 0), 105)]);
        }

        #[test]
        fn mail_is_dropped_once_every_cell_that_could_read_it_has_finished() {
            let mut graph = PhaseGraph::new(Toy);
            let row: Vec<ChunkPos> = (20..24).map(|x| ChunkPos::new(x, 0)).collect();
            finish(&mut graph, &row);
            let source = PhaseId::of::<SourcePhase>();
            let cell = |x| ChunkPos::new(x, 0);
            assert!(!graph.mail_alive(source, cell(21), 0, cell(21)), "its reader finished");
            assert!(graph.mail_alive(source, cell(23), 0, cell(24)), "nothing finished there yet");
            assert!(!graph.mail_alive(source, cell(22), 0, cell(72)), "no cell lists that target");

            graph.sweep_retained();
            for (key, node) in graph.graph.nodes.iter().filter(|(_, node)| node.retained.is_some()) {
                assert!(
                    graph
                        .graph
                        .mail
                        .get(&key)
                        .is_none_or(|mail| mail.out.iter().all(|(target, _)| graph.mail_alive(source, node.cell, node.hop, *target))),
                    "retained nodes keep only mail somebody can still read"
                );
            }
        }

        #[test]
        fn a_node_with_pruned_mail_is_rebuilt_for_a_cell_that_needs_it_again() {
            let mut graph = PhaseGraph::new(Toy);
            let row: Vec<ChunkPos> = (300..304).map(|x| ChunkPos::new(x, 0)).collect();
            finish(&mut graph, &row);
            let first = SEEN.lock().unwrap().iter().find(|(cell, _)| *cell == ChunkPos::new(300, 0)).cloned().expect("the sink ran");
            let runs = |x: i32| SOURCE_RUNS.lock().unwrap().iter().filter(|cell| cell.x == x).count();
            let before = runs(299);

            graph.sweep_retained();
            finish(&mut graph, &[ChunkPos::new(300, 0)]);

            assert!(runs(299) > before, "the retained source had pruned the mail for that cell, so it was built again");
            let again = SEEN.lock().unwrap().iter().rev().find(|(cell, _)| *cell == ChunkPos::new(300, 0)).cloned().expect("the sink ran again");
            assert_eq!(again, first, "the cell sees exactly what it saw the first time");
        }

        #[test]
        fn the_retention_budget_caps_how_many_nodes_of_a_phase_are_kept() {
            let mut graph = PhaseGraph::new(Toy);
            graph.set_focus(&[(10, 10)], 20);
            graph.set_retention_budget(0.01);
            let cells: Vec<ChunkPos> = (0..20).flat_map(|x| (0..20).map(move |z| ChunkPos::new(x, z))).collect();
            finish(&mut graph, &cells);
            assert!(
                graph.retained_count() <= graph.retention_cap(),
                "kept {} against a cap of {}",
                graph.retained_count(),
                graph.retention_cap()
            );
            assert!(graph.retention_cap() < 40, "the cap has to be below the 40 edge nodes for this to test anything");
        }

        #[test]
        fn footprints_follow_the_declared_offsets() {
            let footprints = footprints::<Toy>();
            assert_eq!(footprints[&(PhaseId::of::<SinkPhase>(), 0)], vec![(0, 0)]);
            assert_eq!(footprints[&(PhaseId::of::<SourcePhase>(), 0)], vec![(-1, 0), (0, 0), (1, 0)]);
        }

        #[test]
        fn a_cancelled_request_keeps_its_pipeline_and_a_new_request_reuses_it() {
            let mut graph = PhaseGraph::new(Toy);
            let cell = ChunkPos::new(7, 0);
            graph.request(cell);
            let nodes = graph.graph.nodes.len();
            graph.cancel(cell);
            assert_eq!(graph.abandoned.len(), 1);
            assert_eq!(graph.graph.nodes.len(), nodes, "nothing was torn down");

            graph.request(cell);
            assert!(graph.abandoned.is_empty());
            assert_eq!(graph.graph.nodes.len(), nodes, "the request found the pipeline again");
        }

        #[test]
        fn an_abandoned_pipeline_is_released_once_no_focus_can_reach_it() {
            let mut graph = PhaseGraph::new(Toy);
            graph.set_focus(&[(7, 0)], 4);
            graph.request(ChunkPos::new(7, 0));
            graph.cancel(ChunkPos::new(7, 0));
            graph.set_focus(&[(8, 0)], 4);
            assert_eq!(graph.abandoned.len(), 1, "still within reach of the focus");

            graph.set_focus(&[(500, 500)], 4);
            assert!(graph.abandoned.is_empty());
            assert_eq!(graph.graph.nodes.len(), 0, "the whole pipeline is gone");
        }

        #[test]
        fn mail_for_cells_outside_the_view_region_is_not_kept() {
            let mut graph = PhaseGraph::new(Toy);
            graph.set_focus(&[(0, 0)], 3);
            let source = PhaseId::of::<SourcePhase>();
            assert!(graph.mail_alive(source, ChunkPos::new(2, 0), 0, ChunkPos::new(2, 0)));
            assert!(!graph.mail_alive(source, ChunkPos::new(60, 0), 0, ChunkPos::new(60, 0)));
        }
    }
}
