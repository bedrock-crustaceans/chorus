use crate::command::command_definition::CommandDefinition;
use crate::command::context::CommandContext;
use crate::command::sender::CommandSender;
use crate::const_command;
use crate::level::DimensionId;
use crate::level::dimension::Dimension;
use crate::level::level::Level;
use crate::network::bandwidth::BandwidthTracker;
use crate::server::{ServerMetrics, ServerState};
use crate::utils::process::process_stats;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const STATUS_COMMAND: CommandDefinition = const_command! {
    name: "status",
    description: "Reads back the server's performance.",
    aliases: [],
    permission: CommandPermissionLevelString::GameDirectors,
    overloads: [],
    execute: |context, sender, _| {
        sender.reply("§a---- §rServer status§a ----§r");

        let state = context.resource::<ServerState>();
        sender.reply(format!("§6Uptime: §c{}", format_uptime(state.uptime().as_secs())));

        let metrics = context.resource::<ServerMetrics>();
        let color = tps_color(metrics.tps());

        sender.reply(format!("§6Current TPS: {color}{} ({}%)", format_float(metrics.tps()), format_float(metrics.tick_usage())));
        sender.reply(format!("§6Average TPS: {color}{} ({}%)", format_float(metrics.tps_average()), format_float(metrics.tick_usage_average())));

        let bandwidth = context.resource::<BandwidthTracker>();

        sender.reply(format!("§6Network upload: §c{} kB/s", format_float(bandwidth.average_sent() / 1024.)));
        sender.reply(format!("§6Network download: §c{} kB/s", format_float(bandwidth.average_received() / 1024.)));

        if let Some(stats) = process_stats() {
            // the thread count has no portable source, so the line is skipped where it is unknown
            if let Some(threads) = stats.threads {
                sender.reply(format!("§6Thread count: §c{threads}"));
            }

            sender.reply(format!("§6Total memory: §c{} MB.", format_megabytes(stats.resident_bytes)));
            sender.reply(format!("§6Total virtual memory: §c{} MB.", format_megabytes(stats.virtual_bytes)));
        }

        report_worlds(context, sender);

        Ok(())
    }
};

fn report_worlds(context: &CommandContext, sender: &mut CommandSender) {
    let level = context.resource::<Level>();

    let mut dimensions: Vec<&Dimension> = level.dimensions.values().collect();
    dimensions.sort_by_key(|dimension| dimension.id);

    for dimension in dimensions {
        let entities = context
            .world()
            .iter_entities()
            .filter(|entity| entity.get::<DimensionId>().is_some_and(|id| id.0 == dimension.id))
            .count();

        // chunks are never ticked, so both the ticking count and the time spent on them are zero
        sender.reply(format!(
            "§6World \"{}\": §c{}§a loaded chunks, §c0§a ticking chunks, §c{}§a entities. Time §e0ms",
            dimension.name(),
            format_thousands(dimension.chunk_count()),
            format_thousands(entities)
        ));
    }
}

fn tps_color(tps: f64) -> &'static str {
    if tps < 12. {
        "§c"
    } else if tps < 17. {
        "§6"
    } else {
        "§a"
    }
}

fn format_uptime(seconds: u64) -> String {
    let secs = seconds % 60;
    if seconds < 60 {
        return format!("{secs} seconds");
    }

    let minutes = (seconds % 3_600) / 60;
    if seconds < 3_600 {
        return format!("{minutes} minutes {secs} seconds");
    }

    let hours = (seconds % 86_400) / 3_600;
    if seconds < 86_400 {
        return format!("{hours} hours {minutes} minutes {secs} seconds");
    }

    format!("{} days {hours} hours {minutes} minutes {secs} seconds", seconds / 86_400)
}

fn format_float(value: f64) -> String {
    let rounded = format!("{value:.2}");

    rounded.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn format_megabytes(bytes: u64) -> String {
    let formatted = format!("{:.2}", bytes as f64 / 1024. / 1024.);
    let (whole, fraction) = formatted.split_once('.').unwrap_or((formatted.as_str(), "00"));

    format!("{}.{fraction}", group_thousands(whole))
}

fn format_thousands(value: usize) -> String {
    group_thousands(&value.to_string())
}

fn group_thousands(digits: &str) -> String {
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }

        grouped.push(digit);
    }

    grouped
}
