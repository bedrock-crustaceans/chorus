use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::{ScheduleLabel, SystemSet};
use bevy_ecs::system::SystemId;
use bevy_ecs::world::World;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub const TICK_RATE: f64 = 20.0;
const MAX_TICKS_PER_UPDATE: u32 = 5;

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
}

impl Default for TickClock {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs_f64(1.0 / TICK_RATE),
            accumulated: Duration::ZERO,
            last_instant: Instant::now(),
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
        self.0.push_back(job);
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

    world.resource_mut::<TickClock>().accumulated = if ticks_run == MAX_TICKS_PER_UPDATE { Duration::ZERO } else { accumulated };
}
