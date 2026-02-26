use std::path::Path;
use std::process::exit;

use chrono::Local;
use log::debug;

pub struct Logger {}

impl Logger {
    pub fn setup_logger(log_to_file: bool, log_path: &Path) {
        let dispatch = fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{} {} {}] {}",
                    Local::now().format("%H:%M:%S"),
                    record.target(),
                    record.level(),
                    message
                ))
            })
            .chain(std::io::stdout());

        // Set to log level filter
        // Allow all logs in debug mode
        let mut dispatch = dispatch.level(log::LevelFilter::Trace);

        // Just allow info level logs and below
        #[cfg(not(debug_assertions))]
        let mut dispatch = dispatch.level(log::LevelFilter::Info);

        if log_to_file {
            let log_file = format!(
                "{}/{}.log",
                log_path.display(),
                Local::now().format("%Y-%m-%d_%H-%M-%S")
            );

            dispatch = dispatch.chain(fern::log_file(&log_file).unwrap_or_else(|err| {
                eprintln!("An unexpected Error occurred while trying to add a log file at {log_file:?} to the logger, Err: {err}");
                exit(1)
            }));
        }

        dispatch.apply().unwrap_or_else(|err| {
            eprintln!("An unexpected Error occurred while trying to setup the logger, Err: {err}");
            exit(1);
        });

        debug!("Logger initialized!");
    }
}
