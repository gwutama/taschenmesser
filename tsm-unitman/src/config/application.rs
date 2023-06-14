use serde::Deserialize;
use crate::config::LogLevel;


#[derive(Deserialize, Debug)]
pub struct ApplicationConfiguration {
    pub log_level: Option<LogLevel>,
}
