use serde::Deserialize;
use crate::config::LogLevel;


#[derive(Deserialize, Debug)]
pub struct Application {
    pub log_level: Option<LogLevel>,
}
