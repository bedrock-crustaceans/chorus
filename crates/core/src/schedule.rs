use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::{ScheduleLabel, SystemSet};
use bevy_ecs::system::SystemId;
use bevy_ecs::world::World;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::warn;

pub const TICK_RATE: f64 = 20.0;
const MAX_TICKS_PER_UPDATE: u32 = 5;
const OVERLOAD_WARN_INTERVAL: Duration = Duration::from_secs(5);

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tick;

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TickSet {
    First,
    Update,
    Last,
}

#[derive(Resource, Debug)]
pub struct TickClock {
    pub interval: Duration,
    accumulated: Duration,
    last_instant: Instant,
    last_overload_warning: Option<Instant>,
}

impl Default for TickClock {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs_f64(1.0 / TICK_RATE),
            accumulated: Duration::ZERO,
            last_instant: Instant::now(),
            last_overload_warning: None,
        }
    }
}

impl TickClock {
    pub fn remaining(&self) -> Duration {
        self.interval.saturating_sub(self.accumulated)
    }
}

#[derive(Resource, Default)]
pub struct JobQueue(VecDeque<SystemId>);

impl JobQueue {
    pub fn push(&mut self, job: SystemId) {
        if !self.0.contains(&job) {
            self.0.push_back(job);
        }
    }

    pub fn pop(&mut self) -> Option<SystemId> {
        self.0.pop_front()
    }
}

pub fn run_fixed_tick(world: &mut World) {
    let now = Instant::now();

    let (tick_duration, mut accumulated) = {
        let mut clock = world.resource_mut::<TickClock>();
        let elapsed = now - clock.last_instant;
        clock.accumulated += elapsed;
        clock.last_instant = now;
        (clock.interval, clock.accumulated)
    };

    let mut ticks_run = 0u32;
    while accumulated >= tick_duration && ticks_run < MAX_TICKS_PER_UPDATE {
        accumulated -= tick_duration;
        world.run_schedule(Tick);
        ticks_run += 1;
    }

    let mut clock = world.resource_mut::<TickClock>();
    if ticks_run == MAX_TICKS_PER_UPDATE {
        clock.accumulated = Duration::ZERO;
        if accumulated >= tick_duration && clock.last_overload_warning.is_none_or(|last| now.duration_since(last) >= OVERLOAD_WARN_INTERVAL) {
            clock.last_overload_warning = Some(now);
            warn!("server is overloaded, dropped {} ms of ticks", accumulated.as_millis());
        }
    } else {
        clock.accumulated = accumulated;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_ignores_queued_jobs_and_keeps_order() {
        let mut world = World::new();
        let a = world.register_system(|| {});
        let b = world.register_system(|| {});
        let mut queue = JobQueue::default();
        queue.push(a);
        queue.push(b);
        queue.push(a);
        queue.push(b);
        assert_eq!(queue.pop(), Some(a));
        assert_eq!(queue.pop(), Some(b));
        assert_eq!(queue.pop(), None);
        queue.push(a);
        assert_eq!(queue.pop(), Some(a));
    }
}
