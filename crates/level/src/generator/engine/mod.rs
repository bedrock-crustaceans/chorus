use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::generator::dimension::Generator;
use crate::generator::footprint::{self, CellSet, Footprints, footprints};
use crate::generator::phase::{Outbox, PhaseContext, PhaseHandle, PhaseId, Request};
use crate::generator::tasks::{Outcome, TaskGraph, TaskKey, TaskStatus};
use glam::IVec2;

mod cancel;
#[cfg(feature = "debug")]
mod debug;
mod dispatch;
mod focus;
mod locate;
mod mail;
mod plan;
mod retention;
mod source;
mod stats;
#[cfg(test)]
mod tests;
mod topology;

pub use locate::{Immediate, LocateJob, LocateRun, LocateStep, Located};
pub use mail::{MailConsumer, MailRead};
pub use retention::RetentionSize;

use mail::phase_mail;
use topology::phase_shape;

thread_local! {
    static RUNNING_PHASE: std::cell::Cell<&'static str> = const { std::cell::Cell::new("") };
}

pub fn running_phase() -> &'static str {
    RUNNING_PHASE.try_with(std::cell::Cell::get).unwrap_or("")
}

const REPORT_INTERVAL: Duration = Duration::from_secs(30);
const RERANK_INTERVAL: Duration = Duration::from_millis(250);

pub enum TaskEvent<G: Generator> {
    Finished(TaskKey, Outcome, Outbox),
    Resolved(TaskKey, Vec<Request<G>>, PhaseContext<G>),
    Panicked(TaskKey),
}

pub struct Delivery {
    pub phase: PhaseId,
    pub cell: IVec2,
    pub outcome: Outcome,
}

pub type Dispatch<G> = Option<(PhaseHandle<G>, IVec2, PhaseContext<G>)>;

struct PanicGuard<G: Generator> {
    key: TaskKey,
    completions: Sender<TaskEvent<G>>,
}

impl<G: Generator> Drop for PanicGuard<G> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let _ = self.completions.send(TaskEvent::Panicked(self.key));
        }
    }
}

pub struct Engine<G: Generator> {
    pub generator: Arc<G>,
    pub graph: TaskGraph<G>,
    pub completions_tx: Sender<TaskEvent<G>>,
    pub completions_rx: Receiver<TaskEvent<G>>,
    pub pending_dispatch: BinaryHeap<Reverse<(u64, TaskKey)>>,
    pub next_order: u64,
    pub next_sequence: u64,
    pub focus: Vec<IVec2>,
    pub focus_by_x: Vec<IVec2>,
    pub radius: i32,
    pub retention_budget: f64,
    pub lookahead: i32,
    pub abandoned: FxHashSet<TaskKey>,
    pub mail_reads: FxHashMap<PhaseId, Vec<MailRead>>,
    pub mail_consumers: FxHashMap<PhaseId, Vec<MailConsumer>>,
    pub reach: FxHashMap<PhaseId, f64>,
    pub hops: FxHashMap<PhaseId, u32>,
    pub footprints: Footprints,
    pub regions: FxHashMap<(PhaseId, u32), CellSet>,
    pub unit_regions: FxHashMap<i32, CellSet>,
    pub edge_reach: FxHashMap<(PhaseId, PhaseId), i32>,
    pub demand_reach: FxHashMap<PhaseId, i32>,
    pub extent: f64,
    pub terminal: PhaseId,
    pub completed: FxHashSet<IVec2>,
    pub started: FxHashMap<&'static str, u64>,
    pub last_report: Instant,
    last_rank: Instant,
    pub in_flight: usize,
    pub finished: Vec<(IVec2, Outcome)>,
    pub delivered: Vec<Delivery>,
    pub locates: Vec<LocateRun<G>>,
    pub located: Vec<(u64, Option<Located>)>,
    pub next_locate: u64,
    pub discarded: Vec<Outcome>,
    pub to_release: Vec<TaskKey>,
    pub revision: u64,
}

impl<G: Generator> Engine<G> {
    pub fn new(generator: G) -> Self {
        let (completions_tx, completions_rx) = crossbeam_channel::unbounded();
        let (reach, hops) = phase_shape::<G>();
        let footprints = footprints::<G>();
        let extent = footprint::extent(&footprints);
        let regions = footprints.iter().map(|(&key, cells)| (key, CellSet::new(cells, 0))).collect();
        let (mail_reads, mail_consumers) = phase_mail::<G>();

        Self {
            generator: Arc::new(generator),
            graph: TaskGraph::new(),
            completions_tx,
            completions_rx,
            pending_dispatch: BinaryHeap::new(),
            next_order: 0,
            next_sequence: 0,
            focus: Vec::new(),
            focus_by_x: Vec::new(),
            radius: 0,
            retention_budget: 8.0,
            lookahead: 0,
            abandoned: FxHashSet::default(),
            mail_reads,
            mail_consumers,
            reach,
            hops,
            footprints,
            regions,
            unit_regions: FxHashMap::default(),
            edge_reach: FxHashMap::default(),
            demand_reach: FxHashMap::default(),
            extent,
            terminal: PhaseId::of::<G::Terminal>(),
            completed: FxHashSet::default(),
            started: FxHashMap::default(),
            last_report: Instant::now(),
            last_rank: Instant::now(),
            in_flight: 0,
            finished: Vec::new(),
            delivered: Vec::new(),
            locates: Vec::new(),
            located: Vec::new(),
            next_locate: 0,
            discarded: Vec::new(),
            to_release: Vec::new(),
            revision: 0,
        }
    }

    pub fn request(&mut self, cell: IVec2) {
        self.request_phase(PhaseHandle::of::<G::Terminal>(), cell);
    }

    pub fn request_phase(&mut self, root: PhaseHandle<G>, cell: IVec2) {
        let root_key = (root.phase, cell, 0);

        if let Some(key) = self.key_at(root.phase, cell, 0) {
            let task = &mut self.graph.tasks[key];
            match &task.status {
                TaskStatus::Done(outcome) => {
                    let outcome = outcome.clone();
                    self.deliver(root.phase, cell, outcome);
                }
                _ => {
                    task.requested = true;
                    self.abandoned.remove(&key);
                }
            }
            return;
        }

        let order = self.next_order;
        self.next_order += 1;
        self.insert(root, cell, order);

        let key = self.graph.index[&root_key];
        let task = &mut self.graph.tasks[key];
        match &task.status {
            TaskStatus::Done(outcome) => {
                let outcome = outcome.clone();
                self.deliver(root.phase, cell, outcome);
                self.try_evict(key);
            }
            _ => task.requested = true,
        }
    }
}
