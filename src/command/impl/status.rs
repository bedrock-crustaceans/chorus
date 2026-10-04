use crate::command::command_definition::CommandDefinition;
use crate::command::context::CommandContext;
use crate::command::parameter::{CommandOverload, CommandParameter, CommandParameterType};
use crate::config::Config;
use crate::const_command;
use crate::level::DimensionId;
use crate::level::generator::dimension::Dimension;
use crate::level::level::Level;
use crate::network::bandwidth::BandwidthTracker;
use crate::player::Player;
use crate::server::pregen::Pregen;
use crate::server::{ServerMetrics, ServerState};
use crate::utils::process::process_stats;
use atomicow::CowArc;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;
use bevy_tasks::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool};

const SECTIONS: &str = "performance|network|memory|worlds|storage|all";

pub const STATUS_COMMAND: CommandDefinition = const_command! {
    name: "status",
    description: "Shows how the server is doing",
    aliases: [],
    permission: CommandPermissionLevelString::GameDirectors,
    overloads: [
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("section"),
                    kind: CommandParameterType::String,
                    optional: true
                }
            ])
        }
    ],
    execute: |context, args| {
        let sections: &[Section] = match args.first().map(|arg| arg.to_ascii_lowercase()).as_deref() {
            None => &[Section::Summary],
            Some("all") => &[Section::Performance, Section::Network, Section::Memory, Section::Worlds, Section::Storage],
            Some("performance" | "perf" | "tps") => &[Section::Performance],
            Some("network" | "net") => &[Section::Network],
            Some("memory" | "mem") => &[Section::Memory],
            Some("worlds" | "world") => &[Section::Worlds],
            Some("storage" | "level") => &[Section::Storage],
            Some(other) => return Err(format!("unknown section \"{other}\", use one of {SECTIONS}")),
        };

        let mut report = Report::default();
        for section in sections {
            section.write(context, &mut report);
        }
        if sections == [Section::Summary] {
            report.hint(format!("/status <{SECTIONS}> for more"));
        }
        for line in report.lines {
            context.reply(line);
        }
        Ok(())
    }
};

#[derive(PartialEq)]
enum Section {
    Summary,
    Performance,
    Network,
    Memory,
    Worlds,
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
            Self::Storage => storage(context, report),
        }
    }
}

#[derive(Default)]
struct Report {
    lines: Vec<String>,
}

impl Report {
    fn header(&mut self, title: &str) {
        self.lines.push(format!("§3» §b§l{title}§r"));
    }

    fn row(&mut self, label: &str, value: impl AsRef<str>) {
        self.lines.push(format!("  §7{label}§8: §f{}", value.as_ref()));
    }

    fn hint(&mut self, text: impl AsRef<str>) {
        self.lines.push(format!("  §8§o{}§r", text.as_ref()));
    }
}

fn summary(context: &CommandContext, report: &mut Report) {
    let metrics = context.resource::<ServerMetrics>();
    let level = context.resource::<Level>();
    report.header("Server status");
    report.row("Uptime", format_duration(context.resource::<ServerState>().uptime().as_secs()));
    report.row("TPS", format!("{} {}", tps(metrics.tps()), detail(format!("{} avg, {} of the tick", tps(metrics.tps_average()), usage(metrics.tick_usage())))));
    report.row("Players", ratio(players(context, None), context.resource::<Config>().server.max_players as usize));
    if let Some(stats) = process_stats() {
        report.row("Memory", megabytes(stats.resident_bytes));
    }
    let loaded: usize = level.dimensions().map(Dimension::chunk_count).sum();
    report.row("Chunks", format!("{} loaded {}", thousands(loaded), detail(format!("{} unsaved", thousands(level.unsaved_count())))));
    if let Some(pregen) = context.world().get_resource::<Pregen>() {
        report.row("Pregenerating", pregen.progress());
    }
}

fn performance(context: &CommandContext, report: &mut Report) {
    let metrics = context.resource::<ServerMetrics>();
    report.header("Performance");
    report.row("TPS", format!("{} {}", tps(metrics.tps()), detail(format!("{} avg, {} min", tps(metrics.tps_average()), tps(metrics.tps_min())))));
    report.row(
        "Tick time",
        format!(
            "{} {}",
            milliseconds(metrics.mspt()),
            detail(format!("{} avg, {} max", milliseconds(metrics.mspt_average()), milliseconds(metrics.mspt_max())))
        ),
    );
    report.row("Tick usage", format!("{} {}", usage(metrics.tick_usage()), detail(format!("{} avg", usage(metrics.tick_usage_average())))));
    let threads = |count: Option<usize>| count.map_or_else(|| "§8-".to_owned(), |count| format!("§f{count}"));
    report.row(
        "Pool threads",
        format!(
            "{} §7compute§8, {} §7async§8, {} §7io",
            threads(ComputeTaskPool::try_get().map(|pool| pool.thread_num())),
            threads(AsyncComputeTaskPool::try_get().map(|pool| pool.thread_num())),
            threads(IoTaskPool::try_get().map(|pool| pool.thread_num()))
        ),
    );
}

fn network(context: &CommandContext, report: &mut Report) {
    let config = context.resource::<Config>();
    let bandwidth = context.resource::<BandwidthTracker>();
    report.header("Network");
    report.row("Listening", format!("{}:{} {}", config.network.ip, config.network.port, detail(format!("{:?}", config.network.transport))));
    report.row("Players", ratio(players(context, None), config.server.max_players as usize));
    report.row("Upload", format!("{} §7kB/s", decimal(bandwidth.average_sent() / 1024., 1)));
    report.row("Download", format!("{} §7kB/s", decimal(bandwidth.average_received() / 1024., 1)));
}

fn memory(report: &mut Report) {
    report.header("Memory");
    let Some(stats) = process_stats() else {
        report.hint("process stats are unavailable on this platform");
        return;
    };
    report.row("Resident", megabytes(stats.resident_bytes));
    report.row("Virtual", megabytes(stats.virtual_bytes));
    if let Some(threads) = stats.threads {
        report.row("Threads", threads.to_string());
    }
}

fn worlds(context: &CommandContext, report: &mut Report) {
    let level = context.resource::<Level>();
    let mut dimensions: Vec<&Dimension> = level.dimensions().collect();
    dimensions.sort_by_key(|dimension| dimension.id());

    report.header("Worlds");
    for dimension in dimensions {
        let id = dimension.id();
        let entities = context
            .world()
            .iter_entities()
            .filter(|entity| entity.get::<DimensionId>().is_some_and(|dimension| dimension.0 == id))
            .count();
        let generating = if dimension.has_pending_generation() { "§8, §agenerating" } else { "" };
        report.row(
            dimension.name(),
            format!(
                "{} §7chunks {}§8, §f{} §7players§8, §f{} §7entities",
                thousands(dimension.chunk_count()),
                detail(format!("{} unsaved{generating}", thousands(dimension.unsaved_count()))),
                players(context, Some(id)),
                thousands(entities)
            ),
        );
    }
}

fn storage(context: &CommandContext, report: &mut Report) {
    let level = context.resource::<Level>();
    report.header("Storage");
    report.row("Level", format!("{} {}", level.name, detail(format!("seed {}", level.seed))));
    report.row("Unsaved chunks", thousands(level.unsaved_count()));
    match level.storage() {
        Some(storage) => {
            report.row("Path", storage.path().display().to_string());
            report.row("Database size", megabytes(storage.disk_size()));
            report.row("Pending writes", thousands(storage.pending_writes()));
            report.row("Compacting", flag(storage.is_compacting()));
        }
        None => report.hint("this level is not saved to disk"),
    }
    if let Some(pregen) = context.world().get_resource::<Pregen>() {
        report.row("Pregenerating", pregen.progress());
    }
}

fn players(context: &CommandContext, dimension: Option<i32>) -> usize {
    context
        .world()
        .iter_entities()
        .filter(|entity| entity.contains::<Player>())
        .filter(|entity| dimension.is_none_or(|id| entity.get::<DimensionId>().is_some_and(|dimension| dimension.0 == id)))
        .count()
}

/// Secondary information, dimmed and in brackets.
fn detail(text: impl AsRef<str>) -> String {
    format!("§8(§7{}§8)§f", text.as_ref().replace("§f", "§7"))
}

fn ratio(value: usize, max: usize) -> String {
    format!("{value}§8/§7{max}§f")
}

fn flag(value: bool) -> &'static str {
    if value { "§ayes" } else { "§7no" }
}

fn usage(percent: f64) -> String {
    let color = if percent < 50. {
        "§a"
    } else if percent < 80. {
        "§6"
    } else {
        "§c"
    };
    format!("{color}{}%§f", decimal(percent, 1))
}

fn milliseconds(value: f64) -> String {
    format!("{} ms", decimal(value, 2))
}

fn tps(value: f64) -> String {
    let color = if value < 12. {
        "§c"
    } else if value < 17. {
        "§6"
    } else {
        "§a"
    };
    format!("{color}{}§f", decimal(value, 1))
}

fn format_duration(seconds: u64) -> String {
    let (days, hours, minutes, seconds) = (seconds / 86_400, seconds % 86_400 / 3_600, seconds % 3_600 / 60, seconds % 60);
    match (days, hours, minutes) {
        (0, 0, 0) => format!("{seconds}s"),
        (0, 0, _) => format!("{minutes}m {seconds}s"),
        (0, _, _) => format!("{hours}h {minutes}m {seconds}s"),
        _ => format!("{days}d {hours}h {minutes}m"),
    }
}

fn decimal(value: f64, places: usize) -> String {
    let formatted = format!("{value:.places$}");
    if formatted.contains('.') {
        formatted.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        formatted
    }
}

fn megabytes(bytes: u64) -> String {
    let megabytes = bytes as f64 / 1024. / 1024.;
    if megabytes >= 1024. {
        format!("{} GB", decimal(megabytes / 1024., 2))
    } else {
        format!("{} MB", decimal(megabytes, 1))
    }
}

fn thousands(value: usize) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}
