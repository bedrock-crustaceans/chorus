use crate::config::Config;
use bevy_ecs::system::Res;
use chrono::Local;

use tracing::level_filters::LevelFilter;
use tracing::{Event, Subscriber};
use tracing_appender::rolling;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, FmtContext, FormatEvent, FormatFields},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

struct PrettyFormatter;

impl<S, N> FormatEvent<S, N> for PrettyFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(&self, ctx: &FmtContext<'_, S, N>, mut writer: Writer<'_>, event: &Event<'_>) -> std::fmt::Result {
        let meta = event.metadata();
        let ansi = writer.has_ansi_escapes();

        if ansi {
            write!(&mut writer, "\x1B[36m{}\x1B[0m ", Local::now().format("%H:%M:%S"))?;
        } else {
            write!(&mut writer, "{} ", Local::now().format("%H:%M:%S"))?;
        }

        if ansi {
            write!(&mut writer, "[\x1B[1;33m{}\x1B[0m] ", meta.target())?;
        } else {
            write!(&mut writer, "[{}] ", meta.target())?;
        }

        if ansi {
            let color = match *meta.level() {
                tracing::Level::INFO => "\x1B[34m",
                tracing::Level::WARN => "\x1B[33m",
                tracing::Level::ERROR => "\x1B[31m",
                tracing::Level::DEBUG => "\x1B[35m",
                tracing::Level::TRACE => "\x1B[90m",
            };

            write!(&mut writer, "[{}{}\x1B[0m] ", color, meta.level())?;
        } else {
            write!(&mut writer, "[{}] ", meta.level())?;
        }

        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}

pub fn setup_logger(config: Res<Config>) {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy(config.log_level.clone())
        .add_directive("chorus=debug".parse().unwrap())
        .add_directive("reqwest=warn".parse().unwrap())
        .add_directive("hyper=warn".parse().unwrap())
        .add_directive("h2=warn".parse().unwrap());

    let console_layer = fmt::layer().event_format(PrettyFormatter).with_ansi(true);

    let file_layer = if config.log_to_file {
        let file_path = format!("{}.log", Local::now().format("%Y-%m-%d_%H-%M-%S"));

        let appender = rolling::never(config.logs_directory.display().to_string(), file_path);

        Some(fmt::layer().with_writer(appender).with_ansi(false).event_format(PrettyFormatter))
    } else {
        None
    };

    tracing_subscriber::registry().with(filter).with(console_layer).with(file_layer).init();
}
