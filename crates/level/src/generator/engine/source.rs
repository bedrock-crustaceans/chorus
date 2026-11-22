use std::sync::Arc;
use std::time::Instant;

use bevy_tasks::AsyncComputeTaskPool;
use rustc_hash::FxHashMap;

use crate::chunk::Chunk;
use crate::generator::dimension::{GenerationStats, Generator, WorldGenerator};
use crate::generator::error::PhaseFailure;
use crate::generator::phase::PhaseId;

use super::*;
use glam::IVec2;

impl<G: Generator> WorldGenerator for Engine<G> {
    fn request_chunk(&mut self, x: i32, z: i32) {
        self.request(IVec2::new(x, z));
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
        Engine::set_focus(self, focus, radius);
    }

    fn cancel_chunk(&mut self, x: i32, z: i32) {
        self.cancel(IVec2::new(x, z));
    }

    fn tick(&mut self) -> Vec<(i32, i32, Result<Chunk, Arc<PhaseFailure>>)> {
        let pool = AsyncComputeTaskPool::get();
        while let Ok(completion) = self.completions_rx.try_recv() {
            self.handle_with(completion, pool);
        }
        if self.last_report.elapsed() >= REPORT_INTERVAL {
            self.last_report = Instant::now();
            tracing::debug!(target: "generation", "{}", self.progress_report());
        }

        if pool.thread_num() == 0 {
            if let Some(key) = self.pop_ready() {
                match self.start(key) {
                    Ok(Some((handle, cell, context))) => {
                        let (outcome, pushes) = Self::execute(handle, &self.generator, cell, context);
                        if outcome.is_ok() {
                            self.store_mail(key, pushes);
                        }
                        self.complete(key, outcome);
                    }
                    Ok(None) => {}
                    Err(failure) => self.complete(key, Err(failure)),
                }
            }
        } else {
            self.fill_pool(pool);
        }

        self.advance_locates();

        std::mem::take(&mut self.finished)
            .into_iter()
            .map(|(cell, outcome)| {
                let chunk = outcome.map(|output| {
                    let chunk = output.downcast::<Chunk>().expect("only the terminal phase is ever requested");
                    Arc::try_unwrap(chunk).unwrap_or_else(|shared| (*shared).clone())
                });
                (cell.x, cell.y, chunk)
            })
            .collect()
    }

    fn has_pending_work(&self) -> bool {
        !self.pending_dispatch.is_empty() || self.in_flight > 0 || !self.locates.is_empty() || !self.located.is_empty()
    }

    fn start_locate(&mut self, name: &str, from: (i32, i32), radius: i32) -> Option<u64> {
        Engine::start_locate(self, name, from, radius)
    }

    fn take_located(&mut self) -> Vec<(u64, Option<Located>)> {
        std::mem::take(&mut self.located)
    }

    fn release(&mut self) -> bool {
        self.release_one()
    }

    fn retention_report(&self) -> String {
        let mut live: FxHashMap<PhaseId, usize> = FxHashMap::default();
        for task in self.graph.tasks.values() {
            if task.retained.is_some() {
                *live.entry(self.graph.phases.id(task)).or_default() += 1;
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

    fn retention_sizes(&self) -> Vec<RetentionSize> {
        Engine::retention_sizes(self)
    }

    #[cfg(feature = "debug")]
    fn debug_graph(&self) -> Option<&dyn crate::generator::debug::DebugGraph> {
        Some(self)
    }
}
