use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use crate::configuration::paths;

pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();

    if let Err(error) = paths::log_path() {
        tracing::warn!(%error, "could not create log file");
    }
}

pub fn error(message: &str) {
    tracing::error!("{message}");
    write_log_line("ERROR", message);
}

pub fn warn(message: &str) {
    tracing::warn!("{message}");
    write_log_line("WARN", message);
}

pub fn info(message: &str) {
    tracing::info!("{message}");
}

fn write_log_line(level: &str, message: &str) {
    let path = match paths::log_path() {
        Ok(path) => path,
        Err(_) => return,
    };
    append_line(&path, level, message);
}

fn append_line(path: &PathBuf, level: &str, message: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(
            file,
            "{} {}: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            level,
            message
        );
    }
}
