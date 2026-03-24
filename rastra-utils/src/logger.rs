use std::path::Path;
use std::process::exit;

use chrono::Local;
use fern_colored::colors::{Color, ColoredLevelConfig};

pub struct Logger {}

impl Logger {
    pub fn setup_logger(log_to_file: bool, log_path: &Path) {
        let colors = ColoredLevelConfig::default()
            .info(Color::Blue)
            .warn(Color::Yellow)
            .error(Color::Red);

        let console_log = fern::Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "\x1B[36m{}\x1B[0m [\x1B[1;33m{}\x1B[0m] [{}] {}",
                    Local::now().format("%H:%M:%S"),
                    record.target(),
                    colors.color(record.level()),
                    message
                ))
            })
            .chain(std::io::stdout());

        let mut dispatch = fern::Dispatch::new()
            .level(log::LevelFilter::Trace)
            .chain(console_log);

        if log_to_file {
            let file_log = fern::Dispatch::new().format(move |out, message, record| {
                out.finish(format_args!(
                    "{} [{}] [{}] {}",
                    Local::now().format("%H:%M:%S"),
                    record.target(),
                    record.level(),
                    message
                ))
            });

            let mut file_log = file_log.level(log::LevelFilter::Info);

            let log_file = format!(
                "{}/{}.log",
                log_path.display(),
                Local::now().format("%Y-%m-%d_%H-%M-%S")
            );

            file_log = file_log.chain(fern::log_file(&log_file).unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to add a log file at {log_file:?} to the logger, Err: {err}");
                exit(1)
            }));

            dispatch = dispatch.chain(file_log);
        }

        dispatch.apply().unwrap_or_else(|err| {
            eprintln!("An unexpected Error occurred while trying to setup the logger, Err: {err}");
            exit(1);
        });
    }
}
