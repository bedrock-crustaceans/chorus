use crate::config::Config;
use bedrock::protocol::ProtoVersion;
use bevy_ecs::system::Res;
use chrono::Local;

use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Subscriber};
use tracing_appender::rolling;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, FmtContext, FormatEvent, FormatFields},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

fn plain_tag(tags: &str) -> String {
    if tags.is_empty() { String::new() } else { format!("{tags} ") }
}

struct PrettyFormatter {
    with_target: bool,
}

fn level_color(level: Level) -> &'static str {
    match level {
        Level::ERROR => "\x1b[1;31m",
        Level::WARN => "\x1b[1;33m",
        Level::INFO => "\x1b[1;32m",
        Level::DEBUG => "\x1b[1;35m",
        Level::TRACE => "\x1b[1;90m",
    }
}

impl<S, N> FormatEvent<S, N> for PrettyFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(&self, ctx: &FmtContext<'_, S, N>, mut writer: Writer<'_>, event: &Event<'_>) -> std::fmt::Result {
        let meta = event.metadata();
        let time = Local::now().format("%H:%M:%S");
        let level = *meta.level();
        let mut message = String::new();
        ctx.field_format().format_fields(Writer::new(&mut message), event)?;
        let tags: Vec<&str> = ctx.event_scope().map(|scope| scope.from_root().map(|span| span.name()).collect()).unwrap_or_default();
        let tags = tags.join(":");

        if writer.has_ansi_escapes() {
            let tag = if tags.is_empty() { String::new() } else { format!("\x1b[36m{tags}\x1b[0m ") };
            let tint = match level {
                Level::ERROR => "\x1b[31m",
                Level::WARN => "\x1b[33m",
                _ => "",
            };
            write!(writer, "\x1b[90m{time}\x1b[0m {}{level:>5}\x1b[0m {tag}{tint}{message}\x1b[0m", level_color(level))?;
        } else if self.with_target {
            write!(writer, "{time} {level} [{}] {}{message}", meta.target(), plain_tag(&tags))?;
        } else {
            write!(writer, "{time} {level} {}{message}", plain_tag(&tags))?;
        }

        writeln!(writer)
    }
}

pub fn setup_logger(config: Res<Config>) {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy(config.log.level.clone())
        .add_directive("reqwest=warn".parse().unwrap())
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("h2=warn".parse().unwrap());

    let console_layer = fmt::layer()
        .event_format(PrettyFormatter { with_target: false })
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .with_writer(crate::console::ConsoleWriter::default);

    let file_layer = if config.log.to_file {
        let file_path = format!("{}.log", Local::now().format("%Y-%m-%d_%H-%M-%S"));

        let appender = rolling::never(config.log.directory.display().to_string(), file_path);

        Some(fmt::layer().with_writer(appender).with_ansi(false).event_format(PrettyFormatter { with_target: true }))
    } else {
        None
    };

    tracing_subscriber::registry().with(filter).with(console_layer).with(file_layer).init();
    crate::console::banner(
        &format!("Chorus v{}", env!("CARGO_PKG_VERSION")),
        &format!("{} ({})", crate::network::BedrockProtocol::GAME_VERSION, crate::network::BedrockProtocol::PROTOCOL_VERSION),
    );
}
