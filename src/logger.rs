use crate::OsLog;
use dashmap::DashMap;
use log::{LevelFilter, Log, Metadata, Record};

#[derive(Default)]
pub struct Config {
    pub(crate) subsystem: String,
    pub(crate) log_level: Option<LevelFilter>,
    pub(crate) loggers: DashMap<String, (Option<LevelFilter>, OsLog)>,
}

impl Config {
    pub fn with_subsystem(mut self, subsystem: String) -> Self {
        self.subsystem = subsystem;
        return self;
    }

    /// Only levels at or above `level` will be logged.
    pub fn with_max_level(mut self, level: LevelFilter) -> Self {
        self.log_level = Some(level);
        return self;
    }

    /// Sets or updates the category's level filter.
    pub fn with_category_level_filter(self, category: &str, level: LevelFilter) -> Self {
        self.loggers
            .entry(category.into())
            .and_modify(|(existing_level, _)| *existing_level = Some(level))
            .or_insert((Some(level), OsLog::new(&self.subsystem, category)));

        return self;
    }
}

pub struct OsLogger {
    config: std::sync::OnceLock<Config>,
}

impl OsLogger {
    /// Creates a new logger. You must also call `init` to finalize the set up.
    /// By default the level filter will be set to `LevelFilter::Trace`.
    fn new(config: Config) -> Self {
        Self {
            config: std::sync::OnceLock::from(config),
        }
    }

    fn config(&self) -> &Config {
        self.config.get_or_init(Config::default)
    }
}

static IOS_LOGGER: std::sync::OnceLock<OsLogger> = std::sync::OnceLock::new();

impl Log for OsLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let max_level = self
            .config()
            .loggers
            .get(metadata.target())
            .and_then(|pair| pair.0)
            .unwrap_or_else(log::max_level);

        metadata.level() <= max_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let pair = self
                .config()
                .loggers
                .entry(record.target().into())
                .or_insert((None, OsLog::new(&self.config().subsystem, record.target())));

            let message = std::format!("{}", record.args());
            pair.1.with_level(record.level().into(), &message);
        }
    }

    fn flush(&self) {}
}

pub fn init_once(config: Config) {
    let log_level = config.log_level;
    let logger = IOS_LOGGER.get_or_init(|| OsLogger::new(config));
    if let Err(err) = log::set_logger(logger) {
        log::debug!("oslog: log::set_logger failed: {}", err);
    } else if let Some(level) = log_level {
        log::set_max_level(level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, trace, warn};

    #[test]
    fn test_basic_usage() {
        init_once(
            Config::default()
                .with_subsystem(String::from("com.example.oslog"))
                .with_max_level(LevelFilter::Trace)
                .with_category_level_filter("Settings", LevelFilter::Warn)
                .with_category_level_filter("Database", LevelFilter::Error)
                .with_category_level_filter("Database", LevelFilter::Trace),
        );

        // This will not be logged because of its category's custom level filter.
        info!(target: "Settings", "Info");

        warn!(target: "Settings", "Warn");
        error!(target: "Settings", "Error");

        trace!("Trace");
        debug!("Debug");
        info!("Info");
        warn!(target: "Database", "Warn");
        error!("Error");
    }

    #[test]
    fn test_multiple_initialization() {
        init_once(
            Config::default()
                .with_subsystem(String::from("com.example.oslog"))
                .with_max_level(LevelFilter::Trace)
                .with_category_level_filter("Settings", LevelFilter::Warn)
                .with_category_level_filter("Database", LevelFilter::Error)
                .with_category_level_filter("Database", LevelFilter::Trace),
        );

        init_once(
            Config::default()
                .with_subsystem(String::from("com.example.oslog"))
                .with_max_level(LevelFilter::Trace)
                .with_category_level_filter("Settings", LevelFilter::Warn)
                .with_category_level_filter("Database", LevelFilter::Error)
                .with_category_level_filter("Database", LevelFilter::Trace),
        );

        init_once(
            Config::default()
                .with_subsystem(String::from("com.example.oslog"))
                .with_max_level(LevelFilter::Trace)
                .with_category_level_filter("Settings", LevelFilter::Warn)
                .with_category_level_filter("Database", LevelFilter::Error)
                .with_category_level_filter("Database", LevelFilter::Trace),
        );

        // This will not be logged because of its category's custom level filter.
        info!(target: "Settings", "Info");

        warn!(target: "Settings", "Warn");
        error!(target: "Settings", "Error");

        trace!("Trace");
        debug!("Debug");
        info!("Info");
        warn!(target: "Database", "Warn");
        error!("Error");
    }
}
