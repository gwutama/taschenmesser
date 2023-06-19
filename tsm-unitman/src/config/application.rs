use serde::Deserialize;
use crate::config::{LogLevel};


#[derive(Deserialize, Debug)]
pub struct Application {
    log_level: Option<LogLevel>,
}


impl Application {
    pub fn get_log_level(&self) -> LogLevel {
        return self.log_level.clone().unwrap_or(LogLevel::Info);
    }
}
