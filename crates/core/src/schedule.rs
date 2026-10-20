use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::{ScheduleLabel, SystemSet};
use bevy_ecs::system::{In, IntoSystem, SystemId};
use bevy_ecs::world::World;
use std::any::TypeId;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};
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

#[derive(Clone, Copy, Debug)]
pub struct Budget {
    deadline: Instant,
}

impl Budget {
    pub fn until(deadline: Instant) -> Self {
        Self { deadline }
    }

    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    pub fn is_exhausted(&self) -> bool {
        self.remaining().is_zero()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wake {
    Done,
    Now,
    At(Instant),
}

impl Wake {
    pub fn after(delay: Duration) -> Self {
        Self::At(Instant::now() + delay)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Io,
    Background,
}

const PRIORITIES: usize = 2;

type TaskSystem = SystemId<In<Budget>, Wake>;

struct Slot {
    system: TaskSystem,
    priority: Priority,
    queued: bool,
    timer: Option<Instant>,
}

#[derive(Resource, Default)]
pub struct Scheduler {
    slots: Vec<Slot>,
    index: HashMap<TypeId, usize>,
    ready: [VecDeque<usize>; PRIORITIES],
    timers: BinaryHeap<Reverse<(Instant, usize)>>,
}

impl Scheduler {
    pub fn register<T: 'static, M>(world: &mut World, priority: Priority, system: impl IntoSystem<In<Budget>, Wake, M> + 'static) {
        let system = world.register_system(system);
        world.get_resource_or_insert_with(Scheduler::default).insert::<T>(system, priority);
    }

    fn insert<T: 'static>(&mut self, system: TaskSystem, priority: Priority) {
        let previous = self.index.insert(TypeId::of::<T>(), self.slots.len());
        assert!(previous.is_none(), "task {} is already registered", std::any::type_name::<T>());
        self.slots.push(Slot {
            system,
            priority,
            queued: false,
            timer: None,
        });
    }

    fn slot_of<T: 'static>(&self) -> usize {
        *self.index.get(&TypeId::of::<T>()).unwrap_or_else(|| panic!("task {} is not registered", std::any::type_name::<T>()))
    }

    pub fn wake<T: 'static>(&mut self) {
        let slot = self.slot_of::<T>();
        self.enqueue(slot);
    }

    pub fn wake_at<T: 'static>(&mut self, at: Instant) {
        let slot = self.slot_of::<T>();
        self.set_timer(slot, at);
    }

    fn enqueue(&mut self, slot: usize) {
        let state = &mut self.slots[slot];
        if !state.queued {
            state.queued = true;
            self.ready[state.priority as usize].push_back(slot);
        }
    }

    fn set_timer(&mut self, slot: usize, at: Instant) {
        let state = &mut self.slots[slot];
        if state.timer.is_none_or(|current| at < current) {
            state.timer = Some(at);
            self.timers.push(Reverse((at, slot)));
        }
    }

    fn promote_due(&mut self, now: Instant) {
        while let Some(&Reverse((at, slot))) = self.timers.peek() {
            if at > now {
                break;
            }
            self.timers.pop();
            if self.slots[slot].timer == Some(at) {
                self.slots[slot].timer = None;
                self.enqueue(slot);
            }
        }
    }

    fn pop_ready(&mut self) -> Option<(usize, TaskSystem)> {
        let slot = self.ready.iter_mut().find_map(VecDeque::pop_front)?;
        self.slots[slot].queued = false;
        Some((slot, self.slots[slot].system))
    }

    fn finish(&mut self, slot: usize, wake: Wake) {
        match wake {
            Wake::Done => {}
            Wake::Now => self.enqueue(slot),
            Wake::At(at) => self.set_timer(slot, at),
        }
    }

    pub fn next_wake(&mut self) -> Option<Instant> {
        if self.ready.iter().any(|queue| !queue.is_empty()) {
            return Some(Instant::now());
        }
        while let Some(&Reverse((at, slot))) = self.timers.peek() {
            if self.slots[slot].timer == Some(at) {
                return Some(at);
            }
            self.timers.pop();
        }
        None
    }
}

pub fn run_tasks(world: &mut World, deadline: Instant) {
    while Instant::now() < deadline {
        let next = {
            let mut scheduler = world.resource_mut::<Scheduler>();
            scheduler.promote_due(Instant::now());
            scheduler.pop_ready()
        };
        let Some((slot, system)) = next else { break };
        let wake = world.run_system_with(system, Budget::until(deadline)).unwrap_or_else(|err| {
            warn!("task failed to run: {err}");
            Wake::Done
        });
        world.resource_mut::<Scheduler>().finish(slot, wake);
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
    use bevy_ecs::system::ResMut;

    struct First;
    struct Second;
    struct Background;

    #[derive(Resource, Default)]
    struct Log(Vec<&'static str>);

    fn task(name: &'static str, wake: Wake) -> impl FnMut(In<Budget>, ResMut<Log>) -> Wake {
        move |_, mut log| {
            log.0.push(name);
            wake
        }
    }

    fn world_with_tasks() -> World {
        let mut world = World::new();
        world.init_resource::<Log>();
        Scheduler::register::<First, _>(&mut world, Priority::Io, task("first", Wake::Done));
        Scheduler::register::<Second, _>(&mut world, Priority::Io, task("second", Wake::Done));
        Scheduler::register::<Background, _>(&mut world, Priority::Background, task("background", Wake::Done));
        world
    }

    fn ran(world: &mut World) -> Vec<&'static str> {
        std::mem::take(&mut world.resource_mut::<Log>().0)
    }

    fn later() -> Instant {
        Instant::now() + Duration::from_secs(5)
    }

    #[test]
    fn waking_a_queued_task_again_runs_it_once() {
        let mut world = world_with_tasks();
        let mut scheduler = world.resource_mut::<Scheduler>();
        scheduler.wake::<First>();
        scheduler.wake::<First>();
        run_tasks(&mut world, later());
        assert_eq!(ran(&mut world), ["first"]);
    }

    #[test]
    fn io_tasks_run_before_background_ones_and_keep_wake_order() {
        let mut world = world_with_tasks();
        let mut scheduler = world.resource_mut::<Scheduler>();
        scheduler.wake::<Background>();
        scheduler.wake::<Second>();
        scheduler.wake::<First>();
        run_tasks(&mut world, later());
        assert_eq!(ran(&mut world), ["second", "first", "background"]);
    }

    #[test]
    fn a_task_that_returns_now_runs_again_after_the_others() {
        let mut world = World::new();
        world.init_resource::<Log>();
        let mut runs = 0;
        Scheduler::register::<First, _>(&mut world, Priority::Io, move |_: In<Budget>, mut log: ResMut<Log>| {
            log.0.push("first");
            runs += 1;
            if runs < 2 { Wake::Now } else { Wake::Done }
        });
        Scheduler::register::<Second, _>(&mut world, Priority::Io, task("second", Wake::Done));
        let mut scheduler = world.resource_mut::<Scheduler>();
        scheduler.wake::<First>();
        scheduler.wake::<Second>();
        run_tasks(&mut world, later());
        assert_eq!(ran(&mut world), ["first", "second", "first"]);
    }

    #[test]
    fn a_timer_wakes_the_task_once_it_is_due() {
        let mut world = world_with_tasks();
        let due = Instant::now() + Duration::from_millis(30);
        world.resource_mut::<Scheduler>().wake_at::<First>(due);
        run_tasks(&mut world, later());
        assert!(ran(&mut world).is_empty());
        assert_eq!(world.resource_mut::<Scheduler>().next_wake(), Some(due));
        std::thread::sleep(Duration::from_millis(40));
        run_tasks(&mut world, later());
        assert_eq!(ran(&mut world), ["first"]);
        assert_eq!(world.resource_mut::<Scheduler>().next_wake(), None);
    }

    #[test]
    fn an_earlier_timer_replaces_a_later_one() {
        let mut world = world_with_tasks();
        let mut scheduler = world.resource_mut::<Scheduler>();
        scheduler.wake_at::<First>(Instant::now() + Duration::from_secs(60));
        let early = Instant::now() + Duration::from_millis(10);
        scheduler.wake_at::<First>(early);
        assert_eq!(scheduler.next_wake(), Some(early));
    }

    #[test]
    fn nothing_runs_once_the_deadline_has_passed() {
        let mut world = world_with_tasks();
        world.resource_mut::<Scheduler>().wake::<First>();
        run_tasks(&mut world, Instant::now());
        assert!(ran(&mut world).is_empty());
        assert!(world.resource_mut::<Scheduler>().next_wake().is_some());
    }
}
