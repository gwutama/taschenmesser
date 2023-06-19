use serde::Deserialize;
use users::{get_current_gid, get_current_uid, get_group_by_name, get_user_by_name};

use crate::config::ProcessProbe;
use crate::unit;


#[derive(Deserialize, Debug)]
pub struct Unit {
    name: String,
    executable: String,
    arguments: Option<Vec<String>>,
    dependencies: Option<Vec<String>>,
    restart_policy: Option<unit::RestartPolicy>,
    user: Option<String>,
    group: Option<String>,
    enabled: Option<bool>,
    liveness_probe: Option<ProcessProbe>,
}


impl Unit {
    pub fn get_name(&self) -> String {
        return self.name.clone();
    }

    pub fn get_executable(&self) -> String {
        return self.executable.clone();
    }

    pub fn get_arguments(&self) -> Vec<String> {
        return self.arguments.clone().unwrap_or(Vec::new());
    }

    pub fn get_dependencies(&self) -> Vec<String> {
        return self.dependencies.clone().unwrap_or(Vec::new());
    }

    pub fn get_restart_policy(&self) -> unit::RestartPolicy {
        return self.restart_policy.clone().unwrap_or(unit::RestartPolicy::Always);
    }

    pub fn is_enabled(&self) -> bool {
        return self.enabled.clone().unwrap_or(true);
    }

    pub fn get_liveness_probe(&self) -> Option<unit::ProcessProbeRef> {
        return match &self.liveness_probe {
            Some(liveness_probe) => Some(liveness_probe.build_ref()),
            None => None,
        }
    }

    pub fn build_ref(&self) -> unit::UnitRef {
        return unit::Unit::new_ref(
            self.get_name(),
            self.get_executable(),
            self.get_arguments(),
            self.get_restart_policy(),
            self.get_uid(),
            self.get_gid(),
            self.is_enabled(),
            self.get_liveness_probe(),
        );
    }

    /// If user is valid, return its uid
    /// Otherwise, return own uid
    fn get_uid(&self) -> u32 {
        return match &self.user {
            Some(user) => {
                match get_user_by_name(user) {
                    Some(user) => user.uid(),
                    None => get_current_uid(),
                }
            },
            None => {
                get_current_uid()
            }
        }
    }

    /// If group is valid, return its gid
    /// Otherwise, return own gid
    fn get_gid(&self) -> u32 {
        return match &self.group {
            Some(group) => {
                match get_group_by_name(group) {
                    Some(group) => group.gid(),
                    None => get_current_gid(),
                }
            },
            None => {
                get_current_gid()
            }
        }
    }
}
