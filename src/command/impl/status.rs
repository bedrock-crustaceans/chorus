use crate::Scheduler;
use crate::command::command_definition::{CommandDefinition, CommandFlags};
use crate::command::context::CommandContext;
use crate::command::parameter::{CommandEnum, CommandParameter};
use crate::command::report::{Report, Value, duration_text};
use crate::command::style::CommandStyle;
use crate::config::Config;
use crate::format::{decimal, percent, thousands, tps_tone, usage_tone};
use crate::level::DimensionId;
use crate::level::generator::dimension::Dimension;
use crate::level::level::Level;
use crate::network::bandwidth::BandwidthTracker;
use crate::player::Player;
use crate::player::chunk_view::ChunkView;
use crate::server::pregen::Pregen;
use crate::server::{ServerMetrics, ServerState};
use crate::utils::process::process_stats;
use crate::values;
use crate::world::chunks::ChunkSerializer;
use bevy_tasks::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};
use chorus_core::permission::PermissionLevel;

const SECTION: CommandEnum = CommandEnum::new("StatusSection", values!["performance", "network", "memory", "worlds", "chunks", "tasks", "storage", "all"]);

pub static STATUS_COMMAND: CommandDefinition = CommandDefinition::new("status", "Shows how the server is doing", |context, args| {
    let sections: &[Section] = match args.string("section") {
        None => &[Section::Summary],
        Some("all") => &[Section::Performance, Section::Network, Section::Memory, Section::Worlds, Section::Chunks, Section::Tasks, Section::Storage],
        Some("performance") => &[Section::Performance],
        Some("network") => &[Section::Network],
        Some("memory") => &[Section::Memory],
        Some("worlds") => &[Section::Worlds],
        Some("chunks") => &[Section::Chunks],
        Some("tasks") => &[Section::Tasks],
        Some(_) => &[Section::Storage],
    };

    let mut report = context.report();
    for section in sections {
        section.write(context, &mut report);
    }
    if sections == [Section::Summary] {
        report.hint(format!("/status <{}> for more", SECTION.values.join("|")));
    }
    context.reply_report(report);
    Ok(())
})
.style(CommandStyle::Chorus)
.flags(CommandFlags::HIGHLIGHTED)
.permission(PermissionLevel::Operator)
.overloads(crate::overloads![
    [CommandParameter::enumeration("section", SECTION).optional()],
]);

#[derive(PartialEq)]
enum Section {
    Summary,
    Performance,
    Network,
    Memory,
    Worlds,
    Chunks,
    Tasks,
    Storage,
}

impl Section {
    fn write(&self, context: &CommandContext, report: &mut Report) {
        match self {
            Self::Summary => summary(context, report),
            Self::Performance => performance(context, report),
            Self::Network => network(context, report),
            Self::Memory => memory(report),
            Self::Worlds => worlds(context, report),
            Self::Chunks => chunks(context, report),
            Self::Tasks => tasks(context, report),
            Self::Storage => storage(context, report),
        }
    }
}

fn summary(context: &CommandContext, report: &mut Report) {
    let metrics = context.resource::<ServerMetrics>();
    let level = context.resource::<Level>();
    report.section("Server status", |report| {
        report.row("Uptime", Value::duration(context.resource::<ServerState>().uptime()));
        report.row("TPS", tps_value(metrics.tps()).note(format!("{} avg, {}% of the tick", decimal(metrics.tps_average(), 1), decimal(metrics.tick_usage(), 1))));
        report.row("Players", Value::ratio(players(context, None), context.resource::<Config>().server.max_players as usize));
        if let Some(stats) = process_stats() {
            report.row("Memory", Value::bytes(stats.resident_bytes));
        }
        let loaded: usize = level.dimensions().map(Dimension::chunk_count).sum();
        report.row("Chunks", Value::count(loaded).unit("loaded").note(format!("{} unsaved", level.unsaved_count())));
        if let Some(pregen) = context.world().get_resource::<Pregen>() {
            report.row("Pregenerating", pregen.progress());
        }
    });
}

fn performance(context: &CommandContext, report: &mut Report) {
    let metrics = context.resource::<ServerMetrics>();
    report.section("Performance", |report| {
        report.row("TPS", tps_value(metrics.tps()).note(format!("{} avg, {} min", decimal(metrics.tps_average(), 1), decimal(metrics.tps_min(), 1))));
        report.row(
            "Tick time",
            Value::new(decimal(metrics.mspt(), 2)).unit("ms").note(format!("{} avg, {} max", decimal(metrics.mspt_average(), 2), decimal(metrics.mspt_max(), 2))),
        );
        report.row("Tick usage", Value::percent(metrics.tick_usage()).tone(usage_tone(metrics.tick_usage())).note(format!("{}% avg", decimal(metrics.tick_usage_average(), 1))));
        let threads = |count: Option<usize>, name: &str| Value::new(count.map_or_else(|| "-".to_owned(), |count| count.to_string())).unit(name);
        report.row(
            "Pool threads",
            threads(ComputeTaskPool::try_get().map(|pool| pool.thread_num()), "compute")
                .and(threads(AsyncComputeTaskPool::try_get().map(|pool| pool.thread_num()), "async"))
                .and(threads(IoTaskPool::try_get().map(|pool| pool.thread_num()), "io")),
        );
    });
}

fn network(context: &CommandContext, report: &mut Report) {
    let config = context.resource::<Config>();
    let bandwidth = context.resource::<BandwidthTracker>();
    report.section("Network", |report| {
        report.row("Listening", Value::new(format!("{}:{}", config.network.ip, config.network.port)).note(format!("{:?}", config.network.transport)));
        report.row("Players", Value::ratio(players(context, None), config.server.max_players as usize));
        report.row("Upload", Value::new(decimal(bandwidth.average_sent() / 1024., 1)).unit("kB/s"));
        report.row("Download", Value::new(decimal(bandwidth.average_received() / 1024., 1)).unit("kB/s"));
    });
}

fn memory(report: &mut Report) {
    report.section("Memory", |report| {
        let Some(stats) = process_stats() else {
            report.hint("process stats are unavailable on this platform");
            return;
        };
        report.row("Resident", Value::bytes(stats.resident_bytes));
        report.row("Virtual", Value::bytes(stats.virtual_bytes));
        if let Some(threads) = stats.threads {
            report.row("Threads", Value::new(threads));
        }
    });
}

fn worlds(context: &CommandContext, report: &mut Report) {
    let level = context.resource::<Level>();
    let mut dimensions: Vec<&Dimension> = level.dimensions().collect();
    dimensions.sort_by_key(|dimension| dimension.id());

    report.section("Worlds", |report| {
        for dimension in dimensions {
            let id = dimension.id();
            let entities = context
                .world()
                .iter_entities()
                .filter(|entity| entity.get::<DimensionId>().is_some_and(|dimension| dimension.0 == id))
                .count();
            report.section(dimension.name(), |report| {
                report.row("Chunks", Value::count(dimension.chunk_count()).note(format!("{} unsaved", dimension.unsaved_count())));
                if dimension.has_pending_generation() {
                    report.row("Generating", "yes");
                }
                report.row("Players", Value::count(players(context, Some(id))));
                report.row("Entities", Value::count(entities));
            });
        }
    });
}

fn chunks(context: &CommandContext, report: &mut Report) {
    let level = context.resource::<Level>();
    let serializer = context.resource::<ChunkSerializer>();
    report.section("Chunks", |report| {
        let Some(view) = context.get::<ChunkView>() else {
            report.hint("run this as a player to see the chunk view");
            return;
        };
        let stats = serializer.view_stats(view, level.dimension(view.dimension));
        report.row("View", Value::new(view.radius).unit("radius").note(format!("{} chunks", thousands(stats.wanted))));
        report.row(
            "Generated",
            Value::ratio(stats.generated, stats.wanted).note(format!("{}, radius {} of {}", percent(stats.generated, stats.wanted), decimal(stats.generated_radius as f64, 1), stats.radius)),
        );
        report.row("Sent", Value::ratio(stats.sent, stats.wanted).note(format!("{}, radius {} of {}", percent(stats.sent, stats.wanted), decimal(stats.sent_radius as f64, 1), stats.radius)));
        report.row("Waiting", Value::count(stats.queued).note(format!("{} serialized, {} serializing", stats.ready, stats.in_flight)));
    });
}

fn tasks(context: &CommandContext, report: &mut Report) {
    let mut tasks = context.resource::<Scheduler>().tasks();
    tasks.sort_by_key(|task| (task.priority, std::cmp::Reverse(task.stats.runtime)));
    report.section("Tasks", |report| {
        for task in tasks {
            let state = match (task.queued, task.next_run_in) {
                (true, _) => "ready".to_owned(),
                (false, Some(wait)) => format!("next in {}", duration_text(wait)),
                (false, None) => "idle".to_owned(),
            };
            report.row(
                task.name,
                Value::count(task.stats.runs as usize).unit("runs").and(Value::duration(task.stats.runtime)).note(format!("{:?}, {} avg, {state}", task.priority, duration_text(task.stats.average()))),
            );
        }
    });
}

fn storage(context: &CommandContext, report: &mut Report) {
    let level = context.resource::<Level>();
    report.section("Storage", |report| {
        report.row("Level", Value::new(&level.name).note(format!("seed {}", level.seed)));
        report.row("Unsaved chunks", Value::count(level.unsaved_count()));
        match level.storage() {
            Some(storage) => {
                report.row("Path", storage.path().display().to_string());
                report.row("Database size", Value::bytes(storage.disk_size()));
                report.row("Pending writes", Value::count(storage.pending_writes()));
                report.row("Compacting", if storage.is_compacting() { "yes" } else { "no" });
            }
            None => report.hint("this level is not saved to disk"),
        }
        if let Some(pregen) = context.world().get_resource::<Pregen>() {
            report.row("Pregenerating", pregen.progress());
        }
    });
}

fn tps_value(tps: f64) -> Value {
    Value::new(decimal(tps, 1)).tone(tps_tone(tps))
}

fn players(context: &CommandContext, dimension: Option<i32>) -> usize {
    context
        .world()
        .iter_entities()
        .filter(|entity| entity.contains::<Player>())
        .filter(|entity| dimension.is_none_or(|id| entity.get::<DimensionId>().is_some_and(|dimension| dimension.0 == id)))
        .count()
}
