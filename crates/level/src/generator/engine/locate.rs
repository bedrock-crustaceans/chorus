use rustc_hash::FxHashSet;

use crate::generator::dimension::Generator;
use crate::generator::phase::PhaseId;
use crate::generator::tasks::Outcome;

use super::*;
use glam::IVec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Located {
    pub x: i32,
    pub z: i32,
    pub y: Option<i32>,
    pub position: Option<(i32, i32)>,
}

pub enum LocateStep<G: Generator> {
    Demand(Vec<Request<G>>),
    Done(Option<Located>),
}

pub trait LocateJob<G: Generator>: Send + Sync {
    fn step(&mut self, arrived: Vec<Delivery>) -> LocateStep<G>;
}

pub struct Immediate(pub Option<Located>);

impl<G: Generator> LocateJob<G> for Immediate {
    fn step(&mut self, _arrived: Vec<Delivery>) -> LocateStep<G> {
        LocateStep::Done(self.0)
    }
}

pub struct LocateRun<G: Generator> {
    id: u64,
    job: Box<dyn LocateJob<G>>,
    waiting: FxHashSet<(PhaseId, IVec2)>,
    arrived: Vec<Delivery>,
}

impl<G: Generator> Engine<G> {
    pub fn start_locate(&mut self, name: &str, from: (i32, i32), radius: i32) -> Option<u64> {
        let job = self.generator.locate_job(name, from, radius)?;
        Some(self.start_job(job))
    }

    pub fn start_job(&mut self, job: Box<dyn LocateJob<G>>) -> u64 {
        let id = self.next_locate;
        self.next_locate += 1;
        let mut run = LocateRun {
            id,
            job,
            waiting: FxHashSet::default(),
            arrived: Vec::new(),
        };
        match self.step_locate(&mut run) {
            Some(result) => self.located.push((id, result)),
            None => self.locates.push(run),
        }
        id
    }

    pub fn take_delivered(&mut self) -> Vec<Delivery> {
        std::mem::take(&mut self.delivered)
    }

    pub(super) fn step_locate(&mut self, run: &mut LocateRun<G>) -> Option<Option<Located>> {
        let arrived = std::mem::take(&mut run.arrived);
        match run.job.step(arrived) {
            LocateStep::Done(result) => Some(result),
            LocateStep::Demand(cells) if cells.is_empty() => Some(None),
            LocateStep::Demand(cells) => {
                for Request { handle, cell } in cells {
                    run.waiting.insert((handle.phase, cell));
                    self.request_phase(handle, cell);
                }
                None
            }
        }
    }

    pub(super) fn advance_locates(&mut self) {
        if self.locates.is_empty() {
            return;
        }
        let delivered = std::mem::take(&mut self.delivered);
        for delivery in delivered {
            let mut claimed = false;
            for run in &mut self.locates {
                if run.waiting.remove(&(delivery.phase, delivery.cell)) {
                    claimed = true;
                    run.arrived.push(Delivery {
                        phase: delivery.phase,
                        cell: delivery.cell,
                        outcome: delivery.outcome.clone(),
                    });
                }
            }
            if !claimed {
                self.delivered.push(delivery);
            }
        }
        let mut index = 0;
        while index < self.locates.len() {
            if self.locates[index].waiting.is_empty() {
                let mut run = self.locates.swap_remove(index);
                match self.step_locate(&mut run) {
                    Some(result) => self.located.push((run.id, result)),
                    None => self.locates.push(run),
                }
            } else {
                index += 1;
            }
        }
    }

    pub(super) fn deliver(&mut self, phase: PhaseId, cell: IVec2, outcome: Outcome) {
        if phase == self.terminal {
            self.finished.push((cell, outcome));
        } else {
            self.delivered.push(Delivery { phase, cell, outcome });
        }
    }
}
