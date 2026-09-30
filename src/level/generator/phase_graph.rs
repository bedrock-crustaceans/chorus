use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::Arc;

use bevy_tasks::AsyncComputeTaskPool;
use crossbeam_channel::{Receiver, Sender};

use crate::level::generator::dimension::Generator;
use crate::level::generator::graph::{Graph, Node, NodeKey, NodeStatus};
use crate::level::generator::phase::{PhaseDescriptor, PhaseId, PhaseInputs, PhaseValue, Requirement};
use crate::level::generator::pos::ChunkPos;

pub struct Request<G: Generator> {
    descriptor: PhaseDescriptor<G>,
    cell: ChunkPos,
    notify: Option<Sender<(ChunkPos, G::Value)>>,
}

pub struct RequestHandle<G: Generator>(Sender<Request<G>>);

impl<G: Generator> Clone for RequestHandle<G> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<G: Generator> RequestHandle<G> {
    pub fn request<P: PhaseValue<G>>(&self, cell: ChunkPos, notify: Option<Sender<(ChunkPos, G::Value)>>) {
        let _ = self.0.send(Request {
            descriptor: PhaseDescriptor::of::<P>(),
            cell,
            notify,
        });
    }
}

pub struct PhaseGraph<G: Generator> {
    generator: Arc<G>,
    graph: Graph<G>,
    requests_tx: Sender<Request<G>>,
    requests_rx: Receiver<Request<G>>,
    completions_tx: Sender<(NodeKey, G::Value)>,
    completions_rx: Receiver<(NodeKey, G::Value)>,
    pending_dispatch: BinaryHeap<Reverse<(u64, NodeKey)>>,
    next_priority: u64,
    in_flight: usize,
}

impl<G: Generator> PhaseGraph<G> {
    pub fn new(generator: G) -> Self {
        let (requests_tx, requests_rx) = crossbeam_channel::unbounded();
        let (completions_tx, completions_rx) = crossbeam_channel::unbounded();

        Self {
            generator: Arc::new(generator),
            graph: Graph::new(),
            requests_tx,
            requests_rx,
            completions_tx,
            completions_rx,
            pending_dispatch: BinaryHeap::new(),
            next_priority: 0,
            in_flight: 0,
        }
    }

    pub fn generator(&self) -> &G {
        &self.generator
    }

    pub fn requests(&self) -> RequestHandle<G> {
        RequestHandle(self.requests_tx.clone())
    }

    pub fn node_count(&self) -> usize {
        self.graph.nodes.len()
    }

    pub fn has_pending_work(&self) -> bool {
        !self.pending_dispatch.is_empty() || !self.requests_rx.is_empty() || self.in_flight > 0
    }

    pub fn tick(&mut self) {
        while let Ok((key, output)) = self.completions_rx.try_recv() {
            self.in_flight -= 1;
            self.complete(key, output);
        }

        while let Ok(request) = self.requests_rx.try_recv() {
            self.request(request.descriptor, request.cell, request.notify);
        }

        let max_in_flight = (AsyncComputeTaskPool::get().thread_num() * 2).max(1);
        while self.in_flight < max_in_flight {
            let Some(Reverse((_, key))) = self.pending_dispatch.pop() else { break };
            self.dispatch(key);
        }
    }

    fn push_ready(&mut self, key: NodeKey) {
        self.pending_dispatch.push(Reverse((self.graph.nodes[key].priority, key)));
    }

    fn request(&mut self, descriptor: PhaseDescriptor<G>, cell: ChunkPos, notify: Option<Sender<(ChunkPos, G::Value)>>) {
        enum Step<G: Generator> {
            Enter {
                descriptor: PhaseDescriptor<G>,
                cell: ChunkPos,
                hop: u32,
                notify: Option<Sender<(ChunkPos, G::Value)>>,
            },
            Finalize {
                key: NodeKey,
                dep_keys: Vec<(PhaseId, ChunkPos, u32)>,
            },
        }

        fn dependency_hop<G: Generator>(requirement: &Requirement<G>, current_hop: u32) -> Option<u32> {
            match requirement.self_max_hops {
                Some(max_hops) => (current_hop + 1 <= max_hops).then_some(current_hop + 1),
                None => Some(0),
            }
        }

        let priority = self.next_priority;
        self.next_priority += 1;

        let mut work = vec![Step::Enter { descriptor, cell, hop: 0, notify }];

        while let Some(step) = work.pop() {
            match step {
                Step::Enter { descriptor, cell, hop, notify } => {
                    let map_key = (descriptor.phase, cell, hop);

                    if let Some(&key) = self.graph.index.get(&map_key) {
                        match &self.graph.nodes[key].status {
                            NodeStatus::Done(output) => {
                                if let Some(notify) = notify {
                                    let _ = notify.send((cell, output.clone()));
                                }
                            }
                            _ => {
                                if let Some(notify) = notify {
                                    self.graph.nodes[key].waiters.push(notify);
                                }
                            }
                        }
                        continue;
                    }

                    let key = self.graph.nodes.insert(Node {
                        descriptor,
                        cell,
                        hop,
                        priority,
                        deps: Vec::new(),
                        dependents: Vec::new(),
                        status: NodeStatus::Pending { remaining: 0 },
                        waiters: notify.into_iter().collect(),
                    });
                    self.graph.index.insert(map_key, key);

                    let mut dep_keys = Vec::new();
                    let mut child_enters = Vec::new();
                    for requirement in (descriptor.requires)() {
                        let Some(dependency_hop) = dependency_hop(&requirement, hop) else {
                            continue;
                        };
                        for dependency_cell in (requirement.cells)(cell) {
                            dep_keys.push((requirement.descriptor.phase, dependency_cell, dependency_hop));
                            child_enters.push(Step::Enter {
                                descriptor: requirement.descriptor,
                                cell: dependency_cell,
                                hop: dependency_hop,
                                notify: None,
                            });
                        }
                    }

                    work.push(Step::Finalize { key, dep_keys });
                    work.extend(child_enters);
                }
                Step::Finalize { key, dep_keys } => {
                    let mut deps = Vec::with_capacity(dep_keys.len());
                    for dep_key in dep_keys {
                        let dependency_key = self.graph.index[&dep_key];
                        deps.push(dependency_key);
                        self.graph.nodes[dependency_key].dependents.push(key);
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

    fn dispatch(&mut self, key: NodeKey) {
        let node = &self.graph.nodes[key];
        let descriptor = node.descriptor;
        let cell = node.cell;
        let resolved: Vec<((PhaseId, ChunkPos), G::Value)> = node
            .deps
            .iter()
            .map(|dependency_key| {
                let dependency = &self.graph.nodes[*dependency_key];
                let NodeStatus::Done(output) = &dependency.status else {
                    unreachable!("dependency dispatched before it completed")
                };
                ((dependency.descriptor.phase, dependency.cell), output.clone())
            })
            .collect();

        self.graph.nodes[key].status = NodeStatus::Running;
        self.in_flight += 1;

        let pool = AsyncComputeTaskPool::get();
        if pool.thread_num() > 0 {
            let generator = self.generator.clone();
            let completions = self.completions_tx.clone();
            pool.spawn(async move {
                let inputs = PhaseInputs { resolved: &resolved };
                let output = (descriptor.run)(&generator, cell, &inputs);
                let _ = completions.send((key, output));
            })
            .detach();
        } else {
            let inputs = PhaseInputs { resolved: &resolved };
            let output = (descriptor.run)(&self.generator, cell, &inputs);
            let _ = self.completions_tx.send((key, output));
        }

        for i in 0..self.graph.nodes[key].deps.len() {
            let dependency_key = self.graph.nodes[key].deps[i];
            self.try_evict(dependency_key);
        }
    }

    fn complete(&mut self, key: NodeKey, output: G::Value) {
        let cell = self.graph.nodes[key].cell;
        let waiters = std::mem::take(&mut self.graph.nodes[key].waiters);
        self.graph.nodes[key].status = NodeStatus::Done(output.clone());
        for waiter in waiters {
            let _ = waiter.send((cell, output.clone()));
        }

        for i in 0..self.graph.nodes[key].dependents.len() {
            let dependent_key = self.graph.nodes[key].dependents[i];
            let ready = match &mut self.graph.nodes[dependent_key].status {
                NodeStatus::Pending { remaining } => {
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
    }

    fn try_evict(&mut self, key: NodeKey) {
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

        let node = &self.graph.nodes[key];
        self.graph.index.remove(&(node.descriptor.phase, node.cell, node.hop));
        self.graph.nodes.remove(key);
    }
}
