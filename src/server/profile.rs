use crate::command::context::CommandContext;
use crate::command::report::{Report, Value, duration_text};
use crate::command::style::CommandStyle;
use crate::format::{Tone, decimal, percent, usage_tone};
use crate::schedule::GameSet;
use crate::server::{Server, ServerState};
use crate::{Scheduler, Tick, TickClock, TickSet};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use chorus_core::schedule::{TICK_RATE, TaskInfo};
use std::time::{Duration, Instant};

pub const DEFAULT_PROFILE_SECONDS: u32 = 30;
pub const MAX_PROFILE_SECONDS: u32 = 600;

const MAX_PROFILE_SAMPLES: usize = TICK_RATE as usize * MAX_PROFILE_SECONDS as usize;
const SLOW_PERCENTILES: [(f64, &str); 2] = [(0.95, "95th percentile"), (0.99, "99th percentile")];
const GAME_SETS: [GameSet; 7] = [GameSet::Connection, GameSet::Input, GameSet::World, GameSet::Actors, GameSet::Chat, GameSet::Chunks, GameSet::Broadcast];
const PHASE_COUNT: usize = GAME_SETS.len() + 2;
const PHASE_NAMES: [&str; PHASE_COUNT] = ["Start", "Connection", "Input", "World", "Actors", "Chat", "Chunks", "Broadcast", "Finish"];
const MARK_COUNT: usize = PHASE_COUNT - 1;

pub fn profile_seconds(seconds: u32) -> u32 {
    seconds.clamp(1, MAX_PROFILE_SECONDS)
}

#[derive(Clone, Copy, Debug)]
pub struct TickSample {
    pub total: Duration,
    pub phases: [Duration; PHASE_COUNT],
    pub gap_busy: Duration,
    pub gap_slept: Duration,
}

struct ProfileSession {
    requester: Entity,
    started: Instant,
    ends: Instant,
    first_tick: i64,
    tasks: Vec<TaskInfo>,
    samples: Vec<TickSample>,
}

#[derive(Resource)]
pub struct TickProfile {
    marks: [Instant; MARK_COUNT],
    session: Option<ProfileSession>,
}

impl Default for TickProfile {
    fn default() -> Self {
        Self {
            marks: [Instant::now(); MARK_COUNT],
            session: None,
        }
    }
}

impl TickProfile {
    pub fn is_profiling(&self) -> bool {
        self.session.is_some()
    }

    pub fn start(&mut self, requester: Entity, seconds: u32, tasks: Vec<TaskInfo>, current_tick: i64) {
        let started = Instant::now();
        self.session = Some(ProfileSession {
            requester,
            started,
            ends: started + Duration::from_secs(profile_seconds(seconds) as u64),
            first_tick: current_tick,
            tasks,
            samples: Vec::new(),
        });
    }

    fn mark(index: usize) -> impl FnMut(ResMut<TickProfile>) {
        move |mut profile| profile.marks[index] = Instant::now()
    }

    fn record(mut profile: ResMut<TickProfile>, state: Res<ServerState>, scheduler: Res<Scheduler>) {
        let end = Instant::now();
        let marks = profile.marks;
        let Some(session) = profile
            .session
            .as_mut()
            .filter(|session| state.tick > session.first_tick && session.samples.len() < MAX_PROFILE_SAMPLES)
        else {
            return;
        };

        let mut phases = [Duration::ZERO; PHASE_COUNT];
        let mut previous = state.tick_instant;
        for (phase, mark) in phases.iter_mut().zip(marks) {
            *phase = mark.saturating_duration_since(previous);
            previous = mark.max(previous);
        }
        phases[PHASE_COUNT - 1] = end.saturating_duration_since(previous);

        let gap = scheduler.last_gap();
        session.samples.push(TickSample {
            total: end.saturating_duration_since(state.tick_instant),
            phases,
            gap_busy: gap.duration.saturating_sub(gap.slept),
            gap_slept: gap.slept,
        });
    }

    fn active(profile: Res<TickProfile>) -> bool {
        profile.session.is_some()
    }

    fn due(profile: Res<TickProfile>) -> bool {
        profile.session.as_ref().is_some_and(|session| Instant::now() >= session.ends)
    }
}

pub fn finish_session(world: &mut World) {
    let Some(session) = world.resource_mut::<TickProfile>().session.take() else {
        return;
    };
    let interval = world.resource::<TickClock>().interval;
    let tasks = world.resource::<Scheduler>().tasks();

    let mut context = CommandContext::new(world, session.requester);
    context.set_style(CommandStyle::Chorus);
    let mut report = context.report();
    report.section("Profile", |report| {
        report.row("Duration", Value::duration(session.started.elapsed()));
        write_samples(report, &session.samples, interval);
        write_task_deltas(report, &session.tasks, &tasks);
    });
    context.reply_report(report);
}

fn write_samples(report: &mut Report, samples: &[TickSample], interval: Duration) {
    if samples.is_empty() {
        report.hint("no ticks recorded");
        return;
    }
    let count = samples.len();
    let totals: Vec<Duration> = samples.iter().map(|sample| sample.total).collect();
    let average = mean(&totals);
    let over_budget = totals.iter().filter(|total| **total > interval).count();
    let timed = |duration: Duration| Value::duration(duration).tone(usage_tone(share(duration, interval)));

    report.row("Ticks", Value::count(count).note(format!("{} of the tick budget on average", percent_of(average, interval))));
    report.row("Average", timed(average));
    let mut sorted = totals;
    sorted.sort_unstable();
    for (fraction, label) in SLOW_PERCENTILES {
        report.row(label, timed(sorted[((count as f64 * fraction).ceil() as usize).clamp(1, count) - 1]));
    }
    report.row("Slowest", timed(sorted[count - 1]));
    let over = Value::count(over_budget).note(percent(over_budget, count));
    report.row("Over budget", if over_budget > 0 { over.tone(Tone::Warning) } else { over });

    report.section("Phases", |report| {
        let average_total = average.max(Duration::from_nanos(1));
        for (index, name) in PHASE_NAMES.iter().enumerate() {
            let phases: Vec<Duration> = samples.iter().map(|sample| sample.phases[index]).collect();
            let phase_average = mean(&phases);
            let phase_max = phases.iter().copied().max().unwrap_or_default();
            report.row(
                name,
                Value::duration(phase_average).note(format!("{} of the tick, {} max", percent_of(phase_average, average_total), duration_text(phase_max))),
            );
        }
    });

    let busy: Vec<Duration> = samples.iter().map(|sample| sample.gap_busy).collect();
    let slept: Vec<Duration> = samples.iter().map(|sample| sample.gap_slept).collect();
    report.section("Idle gap", |report| {
        report.row("Asleep", Value::duration(mean(&slept)));
        report.row("Running tasks", Value::duration(mean(&busy)));
    });
}

fn write_task_deltas(report: &mut Report, before: &[TaskInfo], after: &[TaskInfo]) {
    let mut deltas: Vec<(&'static str, u64, Duration)> = after
        .iter()
        .map(|task| {
            let start = before.iter().find(|earlier| earlier.name == task.name).map(|earlier| earlier.stats);
            let runs = task.stats.runs - start.map_or(0, |stats| stats.runs);
            let runtime = task.stats.runtime - start.map_or(Duration::ZERO, |stats| stats.runtime);
            (task.name, runs, runtime)
        })
        .filter(|(_, runs, _)| *runs > 0)
        .collect();
    deltas.sort_by_key(|(_, _, runtime)| std::cmp::Reverse(*runtime));
    report.section("Tasks", |report| {
        if deltas.is_empty() {
            report.hint("no tasks ran");
        }
        for (name, runs, runtime) in deltas {
            report.row(name, Value::count(runs as usize).unit("runs").and(Value::duration(runtime)));
        }
    });
}

fn mean(values: &[Duration]) -> Duration {
    values.iter().sum::<Duration>() / values.len().max(1) as u32
}

fn share(value: Duration, whole: Duration) -> f64 {
    value.as_secs_f64() / whole.as_secs_f64().max(f64::EPSILON) * 100.
}

fn percent_of(value: Duration, whole: Duration) -> String {
    format!("{}%", decimal(share(value, whole), 0))
}

pub struct ProfilePlugin;

impl Plugin for ProfilePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TickProfile>();
        app.add_systems(Tick, TickProfile::mark(0).after(TickSet::First).before(GameSet::Connection).run_if(TickProfile::active));
        for (index, set) in GAME_SETS.iter().enumerate() {
            let mark = TickProfile::mark(index + 1).after(*set).run_if(TickProfile::active);
            match GAME_SETS.get(index + 1) {
                Some(next) => app.add_systems(Tick, mark.before(*next)),
                None => app.add_systems(Tick, mark.before(TickSet::Last)),
            };
        }
        app.add_systems(Tick, TickProfile::record.in_set(TickSet::Last).after(Server::end_tick).run_if(TickProfile::active));
        app.add_systems(Tick, finish_session.run_if(TickProfile::due).in_set(TickSet::Last).after(TickProfile::record));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::schedule::Schedule;

    fn sample(total_ms: u64) -> TickSample {
        TickSample {
            total: Duration::from_millis(total_ms),
            phases: [Duration::from_millis(total_ms) / PHASE_COUNT as u32; PHASE_COUNT],
            gap_busy: Duration::from_millis(1),
            gap_slept: Duration::from_millis(10),
        }
    }

    fn profiling_app() -> App {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Tick));
        app.configure_sets(Tick, (TickSet::First, TickSet::Update, TickSet::Last).chain());
        app.configure_sets(
            Tick,
            (GameSet::Connection, GameSet::Input, GameSet::World, GameSet::Actors, GameSet::Chat, GameSet::Chunks, GameSet::Broadcast)
                .chain()
                .in_set(TickSet::Update),
        );
        app.insert_resource(ServerState::new())
            .init_resource::<Scheduler>()
            .init_resource::<TickClock>()
            .add_plugins(ProfilePlugin);
        app.add_systems(
            Tick,
            (|mut state: ResMut<ServerState>| {
                state.tick += 1;
                state.tick_instant = Instant::now();
            })
            .in_set(TickSet::First),
        );
        app.add_systems(Tick, (|| std::thread::sleep(Duration::from_millis(5))).in_set(GameSet::World));
        app
    }

    #[test]
    fn the_duration_is_clamped() {
        assert_eq!(profile_seconds(0), 1);
        assert_eq!(profile_seconds(u32::MAX), MAX_PROFILE_SECONDS);
    }

    #[test]
    fn the_report_counts_ticks_over_budget() {
        let mut report = Report::new(CommandStyle::Chorus);
        write_samples(&mut report, &[sample(10), sample(60)], Duration::from_millis(50));
        let lines = report.into_lines();
        assert!(lines.iter().any(|line| line.contains("Over budget") && line.contains("§61") && line.contains("50%")), "{lines:#?}");
        assert!(lines.iter().any(|line| line.contains("Slowest") && line.contains("60")), "{lines:#?}");
    }

    #[test]
    fn nothing_is_recorded_without_a_session() {
        let mut app = profiling_app();
        app.world_mut().run_schedule(Tick);
        assert!(!app.world().resource::<TickProfile>().is_profiling());
    }

    #[test]
    fn every_tick_is_split_into_phases_that_add_up_to_its_total() {
        let mut app = profiling_app();
        app.world_mut().resource_mut::<TickProfile>().start(Entity::PLACEHOLDER, 60, Vec::new(), 0);

        app.world_mut().run_schedule(Tick);

        let session = app.world().resource::<TickProfile>().session.as_ref().unwrap();
        assert_eq!(session.samples.len(), 1);
        let sample = session.samples[0];
        let world_phase = sample.phases[PHASE_NAMES.iter().position(|name| *name == "World").unwrap()];
        assert!(world_phase >= Duration::from_millis(5), "{world_phase:?}");
        assert_eq!(sample.phases.iter().sum::<Duration>(), sample.total);
    }

    #[test]
    fn the_tick_a_session_starts_in_is_not_sampled() {
        let mut app = profiling_app();
        app.world_mut().resource_mut::<TickProfile>().start(Entity::PLACEHOLDER, 60, Vec::new(), 1);

        app.world_mut().run_schedule(Tick);

        assert!(app.world().resource::<TickProfile>().session.as_ref().unwrap().samples.is_empty());
    }
}
