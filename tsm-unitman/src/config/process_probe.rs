use serde::Deserialize;

use crate::unit;

#[derive(Deserialize, Debug, Clone)]
/// timeout_s: 0 means no timeout
/// interval_s: 0 means no interval (run once)
pub struct ProcessProbe {
    executable: String,
    arguments: Option<Vec<String>>,
    timeout_s: Option<i32>,
    interval_s: Option<i32>,
}


impl ProcessProbe {
    pub fn get_executable(&self) -> String {
        return self.executable.clone();
    }

    pub fn get_arguments(&self) -> Vec<String> {
        return self.arguments.clone().unwrap_or(Vec::new());
    }

    pub fn get_timeout_s(&self) -> i32 {
        return self.timeout_s.unwrap_or(10);
    }

    pub fn get_interval_s(&self) -> i32 {
        return self.interval_s.unwrap_or(60);
    }

    pub fn build(&self, name: String) -> unit::LivenessProbe {
        return unit::LivenessProbe::new(
            name,
            self.get_executable(),
            self.get_arguments(),
            self.get_timeout_s(),
            self.get_interval_s(),
        );
    }
}