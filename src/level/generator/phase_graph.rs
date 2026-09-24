use std::marker::PhantomData;
use std::sync::Arc;

use bevy_tasks::ComputeTaskPool;
use crossbeam_channel::{Receiver, Sender};

use crate::level::generator::graph::{Graph, Node, NodeKey, NodeStatus};
use crate::level::generator::phase::{ErasedOutput, Phase, PhaseDescriptor, PhaseId, PhaseInputs};
use crate::level::generator::pos::ChunkPos;

pub struct Request<G: 'static> {
    descriptor: PhaseDescriptor<G>,
    cell: ChunkPos,
    notify: Option<Sender<(ChunkPos, ErasedOutput)>>,
}

pub struct RequestHandle<G: 'static>(Sender<Request<G>>);

impl<G: 'static> Clone for RequestHandle<G> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<G: Send + Sync + 'static> RequestHandle<G> {
    pub fn request<P: Phase<G>>(&self, cell: ChunkPos, notify: Option<Sender<(ChunkPos, ErasedOutput)>>) {
        let _ = self.0.send(Request {
            descriptor: PhaseDescriptor::of::<P>(),
            cell,
            notify,
        });
    }
}

pub struct PhaseGraph<G: 'static> {
    generator: Arc<G>,
    graph: Graph<G>,
    requests_tx: Sender<Request<G>>,
    requests_rx: Receiver<Request<G>>,
    completions_tx: Sender<(NodeKey, ErasedOutput)>,
    completions_rx: Receiver<(NodeKey, ErasedOutput)>,
    pending_dispatch: Vec<NodeKey>,
}

impl<G: Send + Sync + 'static> PhaseGraph<G> {
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
            pending_dispatch: Vec::new(),
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

    pub fn tick(&mut self) {
        while let Ok((key, output)) = self.completions_rx.try_recv() {
            self.complete(key, output);
        }

        while let Ok(request) = self.requests_rx.try_recv() {
            self.request(request.descriptor, request.cell, request.notify);
        }

        while let Some(key) = self.pending_dispatch.pop() {
            self.dispatch(key);
        }
    }

    fn request(&mut self, descriptor: PhaseDescriptor<G>, cell: ChunkPos, notify: Option<Sender<(ChunkPos, ErasedOutput)>>) {
        enum Step<G: 'static> {
            Enter {
                descriptor: PhaseDescriptor<G>,
                cell: ChunkPos,
                notify: Option<Sender<(ChunkPos, ErasedOutput)>>,
            },
            Finalize {
                key: NodeKey,
            },
        }

        let mut work = vec![Step::Enter { descriptor, cell, notify }];

        while let Some(step) = work.pop() {
            match step {
                Step::Enter { descriptor, cell, notify } => {
                    let map_key = (descriptor.phase, cell);

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
                        deps: Vec::new(),
                        dependents: Vec::new(),
                        status: NodeStatus::Pending { remaining: 0 },
                        waiters: notify.into_iter().collect(),
                    });
                    self.graph.index.insert(map_key, key);

                    work.push(Step::Finalize { key });
                    for requirement in (descriptor.requires)() {
                        for dependency_cell in (requirement.cells)(cell) {
                            work.push(Step::Enter {
                                descriptor: requirement.descriptor,
                                cell: dependency_cell,
                                notify: None,
                            });
                        }
                    }
                }
                Step::Finalize { key } => {
                    let descriptor = self.graph.nodes[key].descriptor;
                    let cell = self.graph.nodes[key].cell;

                    let mut deps = Vec::new();
                    for requirement in (descriptor.requires)() {
                        for dependency_cell in (requirement.cells)(cell) {
                            let dependency_key = self.graph.index[&(requirement.descriptor.phase, dependency_cell)];
                            deps.push(dependency_key);
                            self.graph.nodes[dependency_key].dependents.push(key);
                        }
                    }

                    let remaining = deps.iter().filter(|dependency_key| !matches!(self.graph.nodes[**dependency_key].status, NodeStatus::Done(_))).count();
                    self.graph.nodes[key].deps = deps;

                    if remaining == 0 {
                        self.pending_dispatch.push(key);
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
        let resolved: Vec<((PhaseId, ChunkPos), ErasedOutput)> = node
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

        let generator = self.generator.clone();
        let completions = self.completions_tx.clone();
        ComputeTaskPool::get()
            .spawn(async move {
                let inputs = PhaseInputs {
                    resolved: &resolved,
                    marker: PhantomData,
                };
                let output = (descriptor.run)(&generator, cell, &inputs);
                let _ = completions.send((key, output));
            })
            .detach();

        let deps = self.graph.nodes[key].deps.clone();
        for dependency_key in deps {
            self.try_evict(dependency_key);
        }
    }

    fn complete(&mut self, key: NodeKey, output: ErasedOutput) {
        let cell = self.graph.nodes[key].cell;
        let waiters = std::mem::take(&mut self.graph.nodes[key].waiters);
        self.graph.nodes[key].status = NodeStatus::Done(output.clone());
        for waiter in waiters {
            let _ = waiter.send((cell, output.clone()));
        }

        let dependents = self.graph.nodes[key].dependents.clone();
        for dependent_key in dependents {
            let ready = match &mut self.graph.nodes[dependent_key].status {
                NodeStatus::Pending { remaining } => {
                    *remaining -= 1;
                    *remaining == 0
                }
                _ => false,
            };
            if ready {
                self.pending_dispatch.push(dependent_key);
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
        self.graph.index.remove(&(node.descriptor.phase, node.cell));
        self.graph.nodes.remove(key);
    }
}
