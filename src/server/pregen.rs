use crate::level::Level;
use bevy_ecs::prelude::{Commands, ResMut, Resource};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tracing::{info, warn};

const MAX_IN_FLIGHT: usize = 512;
const REPORT_INTERVAL: Duration = Duration::from_secs(5);
const STALL_TICKS: u32 = 100;
const MAX_PENDING_WRITES: usize = 50_000;
pub const MAX_CHUNKS: usize = 4_000_000;

#[derive(Resource)]
pub struct Pregen {
    dimension: i32,
    queue: Vec<(i32, i32)>,
    in_flight: HashSet<(i32, i32)>,
    total: usize,
    done: usize,
    started: Instant,
    last_report: Instant,
    stalled_ticks: u32,
}

impl Pregen {
    pub fn new(dimension: i32, (min_x, min_z): (i32, i32), (max_x, max_z): (i32, i32)) -> Self {
        let center = ((min_x + max_x) as f64 / 2.0, (min_z + max_z) as f64 / 2.0);
        let mut queue: Vec<(i32, i32)> = (min_x..=max_x).flat_map(|x| (min_z..=max_z).map(move |z| (x, z))).collect();
        let distance = |&(x, z): &(i32, i32)| (x as f64 - center.0).powi(2) + (z as f64 - center.1).powi(2);
        queue.sort_by(|a, b| distance(b).total_cmp(&distance(a)));
        let now = Instant::now();
        Self {
            dimension,
            total: queue.len(),
            queue,
            in_flight: HashSet::new(),
            done: 0,
            started: now,
            last_report: now,
            stalled_ticks: 0,
        }
    }

    pub fn chunk_count(width: i64, depth: i64) -> usize {
        usize::try_from(width.max(0) * depth.max(0)).unwrap_or(usize::MAX)
    }

    pub fn progress(&self) -> String {
        let elapsed = self.started.elapsed().as_secs_f64();
        let rate = if elapsed > 0.0 { self.done as f64 / elapsed } else { 0.0 };
        let remaining = self.total - self.done;
        let eta = if rate > 0.0 {
            format!(", about {} left", format_duration(remaining as f64 / rate))
        } else {
            String::new()
        };
        format!(
            "{}/{} chunks ({:.1}%), {:.0} chunks/s{eta}",
            self.done,
            self.total,
            self.done as f64 * 100.0 / self.total.max(1) as f64,
            rate
        )
    }

    pub fn run(mut commands: Commands, pregen: Option<ResMut<Pregen>>, mut level: ResMut<Level>) {
        let Some(mut pregen) = pregen else { return };
        let writes_backed_up = level.storage().is_some_and(|storage| storage.pending_writes() > MAX_PENDING_WRITES);
        if writes_backed_up && let Some(storage) = level.storage() {
            storage.schedule_flush();
        }
        let Some(dimension) = level.dimension_mut(pregen.dimension) else {
            warn!("stopping pregeneration, dimension {} is gone", pregen.dimension);
            commands.remove_resource::<Pregen>();
            return;
        };

        let before = pregen.done;
        let finished: Vec<(i32, i32)> = pregen.in_flight.iter().copied().filter(|&(x, z)| dimension.get_chunk(x, z).is_some()).collect();
        for position in finished {
            pregen.in_flight.remove(&position);
            pregen.done += 1;
        }

        while !writes_backed_up && pregen.in_flight.len() < MAX_IN_FLIGHT {
            let Some((x, z)) = pregen.queue.pop() else { break };
            if dimension.get_chunk(x, z).is_some() {
                pregen.done += 1;
                continue;
            }
            dimension.request_chunk(x, z);
            pregen.in_flight.insert((x, z));
        }

        if pregen.done == before && !pregen.in_flight.is_empty() && !writes_backed_up {
            pregen.stalled_ticks += 1;
            if pregen.stalled_ticks >= STALL_TICKS {
                pregen.stalled_ticks = 0;
                let positions: Vec<(i32, i32)> = pregen.in_flight.iter().copied().collect();
                dimension.request_chunks(&positions);
            }
        } else {
            pregen.stalled_ticks = 0;
        }

        if pregen.done >= pregen.total {
            let saved = level.save();
            info!(
                "pregeneration finished: {} chunks in {}, saved {saved}",
                pregen.total,
                format_duration(pregen.started.elapsed().as_secs_f64())
            );
            commands.remove_resource::<Pregen>();
            return;
        }

        if pregen.last_report.elapsed() >= REPORT_INTERVAL {
            pregen.last_report = Instant::now();
            info!("pregenerating: {}", pregen.progress());
        }
    }
}

fn format_duration(seconds: f64) -> String {
    let seconds = seconds.round() as u64;
    match seconds {
        0..60 => format!("{seconds}s"),
        60..3600 => format!("{}m {}s", seconds / 60, seconds % 60),
        _ => format!("{}h {}m", seconds / 3600, seconds % 3600 / 60),
    }
}
