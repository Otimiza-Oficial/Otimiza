// Logger utility
// Centralized logging system

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

pub struct Logger;

impl Logger {
    pub fn log(level: LogLevel, message: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        println!("[{}] [{}] {}", timestamp, level, message);
    }


    pub fn info(message: &str) {
        Self::log(LogLevel::Info, message);
    }

    pub fn warn(message: &str) {
        Self::log(LogLevel::Warn, message);
    }

    pub fn error(message: &str) {
        Self::log(LogLevel::Error, message);
    }
}
