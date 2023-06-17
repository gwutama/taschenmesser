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
    pub fn build_ref(&self) -> unit::ProcessProbeRef {
        let arguments = match &self.arguments {
            Some(arguments) => arguments.clone(),
            None => Vec::new(),
        };

        let timeout_s = match &self.timeout_s {
            Some(timeout_s) => *timeout_s,
            None => 10,
        };

        let interval_s = match &self.interval_s {
            Some(interval_s) => *interval_s,
            None => 60,
        };

        return unit::ProcessProbe::new_ref(
            self.executable.clone(),
            arguments,
            timeout_s,
            interval_s,
        );
    }
}