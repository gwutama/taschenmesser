use std::str::FromStr;
use serde::Deserialize;
use serde::de::Error;


#[derive(Debug, PartialEq, Clone)]
pub enum UnitState {
    Starting,
    Running,
    RunningAndHealthy,
    RunningButDegraded,
    Stopping,
    Stopped,
}


impl<'de> Deserialize<'de> for UnitState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        UnitState::from_str(&s).map_err(Error::custom)
    }
}


impl FromStr for UnitState {
    type Err = String;

    fn from_str(unit_state: &str) -> Result<Self, Self::Err> {
        match unit_state.to_lowercase().as_str() {
            "starting" => Ok(UnitState::Starting),
            "running" => Ok(UnitState::Running),
            "running (healthy)" => Ok(UnitState::RunningAndHealthy),
            "running (degraded)" => Ok(UnitState::RunningButDegraded),
            "stopping" => Ok(UnitState::Stopping),
            "stopped" => Ok(UnitState::Stopped),
            _ => Err(format!("Invalid unit state: {}", unit_state)),
        }
    }
}
