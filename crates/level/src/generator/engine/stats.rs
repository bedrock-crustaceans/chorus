use rustc_hash::FxHashMap;

use crate::generator::dimension::{GenerationStats, Generator};
use crate::generator::tasks::TaskStatus;

use super::*;

impl<G: Generator> Engine<G> {
    pub(super) fn generation_stats(&self) -> GenerationStats {
        let mut live: FxHashMap<&'static str, usize> = FxHashMap::default();
        let mut waiting = 0;
        for task in self.graph.tasks.values() {
            if task.retained.is_some() {
                *live.entry(self.graph.phases.name(task)).or_default() += 1;
            }
            if matches!(task.status, TaskStatus::Done(_)) && task.retained.is_none() {
                waiting += 1;
            }
        }
        let mut retained: Vec<(&'static str, usize)> = live.into_iter().collect();
        retained.sort_unstable();
        let mut started: Vec<(&'static str, u64)> = self.started.iter().map(|(&name, &count)| (name, count)).collect();
        started.sort_unstable();
        GenerationStats {
            tasks: self.graph.tasks.len(),
            waiting,
            discarded: self.discarded.len(),
            completed: self.completed.len(),
            retained,
            started,
        }
    }

    pub(super) fn progress_report(&self) -> String {
        let stats = self.generation_stats();
        let list = |items: Vec<String>| items.join(" ");
        format!(
            "tasks {} done-unretained {} discarded {} retained [{}] chunks completed {} started {}",
            stats.tasks,
            stats.waiting,
            stats.discarded,
            list(stats.retained.iter().map(|(name, count)| format!("{name}={count}")).collect()),
            stats.completed,
            list(stats.started.iter().map(|(name, count)| format!("{name}={count}")).collect())
        )
    }
}
