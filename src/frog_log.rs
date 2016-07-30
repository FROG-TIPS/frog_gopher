extern crate log;

use time;
use std::thread;
use log::{LogRecord,LogLevel,LogLevelFilter,LogMetadata,SetLoggerError};


struct FrogLogger;

impl log::Log for FrogLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Info
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            let thread = thread::current();
            let thread_name = thread.name().unwrap_or("");
            let now = time::now_utc();
            let formatted_now = now.rfc3339();
            println!("{} {} {} {}", formatted_now, thread_name, record.level(), record.args());
        }
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Info);
        Box::new(FrogLogger)
    })
}
