use rustc_hash::{FxHashMap, FxHashSet};

use crate::generator::dimension::Generator;
use crate::generator::phase::{InboxEntry, Outbox, PhaseHandle, PhaseId};
use crate::generator::tasks::{TaskKey, TaskMail, TaskStatus};

use super::*;
use glam::IVec2;

pub struct MailRead {
    pub producer: PhaseId,
    pub offsets: &'static [(i32, i32)],
    pub targets: &'static [(i32, i32)],
    pub max_hops: Option<u32>,
}

pub struct MailConsumer {
    pub phase: PhaseId,
    pub offsets: &'static [(i32, i32)],
    pub targets: &'static [(i32, i32)],
    pub max_hops: Option<u32>,
}

pub(super) fn phase_mail<G: Generator>() -> (FxHashMap<PhaseId, Vec<MailRead>>, FxHashMap<PhaseId, Vec<MailConsumer>>) {
    let mut reads: FxHashMap<PhaseId, Vec<MailRead>> = FxHashMap::default();
    let mut consumers: FxHashMap<PhaseId, Vec<MailConsumer>> = FxHashMap::default();
    let mut seen = FxHashSet::default();
    let mut stack = vec![PhaseHandle::of::<G::Terminal>()];
    while let Some(handle) = stack.pop() {
        if !seen.insert(handle.phase) {
            continue;
        }
        for dependency in (handle.dependencies)() {
            if dependency.handle.phase != handle.phase {
                stack.push(dependency.handle);
            }
            let Some(targets) = dependency.mail_for else { continue };
            reads.entry(handle.phase).or_default().push(MailRead {
                producer: dependency.handle.phase,
                offsets: dependency.offsets,
                targets,
                max_hops: dependency.max_hops,
            });
            consumers.entry(dependency.handle.phase).or_default().push(MailConsumer {
                phase: handle.phase,
                offsets: dependency.offsets,
                targets,
                max_hops: dependency.max_hops,
            });
        }
    }
    (reads, consumers)
}

impl<G: Generator> Engine<G> {
    pub(super) fn release_mail_read_by(&mut self, key: TaskKey) {
        let task = &self.graph.tasks[key];
        let (phase, cell, hop) = (self.graph.phases.id(task), task.cell, task.hop);
        let Some(reads) = self.mail_reads.get(&phase) else { return };
        let mut producers = Vec::new();
        for read in reads {
            let producer_hop = match (read.producer == phase, read.max_hops) {
                (false, _) => 0,
                (true, Some(max_hops)) if hop < max_hops => hop + 1,
                _ => continue,
            };
            for &(dx, dz) in read.offsets {
                let source = cell + IVec2::new(dx, dz);
                if let Some(producer) = self.key_at(read.producer, source, producer_hop) {
                    producers.push(producer);
                }
            }
        }
        for producer in producers {
            self.prune_mail(producer);
        }
    }

    pub(super) fn collect_mail(&self, phase: PhaseId, cell: IVec2, hop: u32, producers: &FxHashMap<(PhaseId, IVec2, u32), TaskKey>) -> Vec<InboxEntry> {
        let mut inbox = Vec::new();
        let Some(reads) = self.mail_reads.get(&phase) else { return inbox };
        for read in reads {
            let dependency_hop = match (read.producer == phase, read.max_hops) {
                (false, _) => 0,
                (true, Some(max_hops)) if hop < max_hops => hop + 1,
                _ => continue,
            };
            for &(dx, dz) in read.offsets {
                let source = cell + IVec2::new(dx, dz);
                let Some(&producer) = producers.get(&(read.producer, source, dependency_hop)) else {
                    continue;
                };
                let Some(mail) = self.graph.mail.get(&producer) else { continue };
                for (target, value) in &mail.out {
                    if read.targets.iter().any(|&(tx, tz)| cell + IVec2::new(tx, tz) == *target) {
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

    pub(super) fn store_mail(&mut self, key: TaskKey, mail: Outbox) {
        if mail.is_empty() || !self.graph.tasks.contains_key(key) {
            return;
        }
        self.graph.mail.insert(key, TaskMail { out: mail, pruned: Vec::new() });
    }

    pub(super) fn lacks_mail_for(&self, key: TaskKey, reader: Option<(IVec2, &'static [(i32, i32)])>) -> bool {
        let Some((consumer, targets)) = reader else { return false };
        let Some(mail) = self.graph.mail.get(&key) else { return false };
        mail.pruned.iter().any(|pruned| targets.iter().any(|&(dx, dz)| consumer + IVec2::new(dx, dz) == *pruned))
    }

    pub(super) fn mail_alive(&self, phase: PhaseId, cell: IVec2, hop: u32, target: IVec2) -> bool {
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
                let reader = cell - IVec2::new(ox, oz);
                if !consumer.targets.iter().any(|&(tx, tz)| reader + IVec2::new(tx, tz) == target) {
                    continue;
                }
                for reader_hop in hops.clone() {
                    match self.node_at(consumer.phase, reader, reader_hop).map(|task| &task.status) {
                        Some(TaskStatus::Done(_)) => {}
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

    pub(super) fn prune_mail(&mut self, key: TaskKey) {
        let Some(task) = self.graph.tasks.get(key) else { return };
        let Some(mail) = self.graph.mail.get(&key) else { return };
        if mail.out.is_empty() {
            return;
        }
        let (phase, cell, hop) = (self.graph.phases.id(task), task.cell, task.hop);
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
}
