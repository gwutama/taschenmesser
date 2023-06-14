use serde::Deserialize;
use users::{get_current_gid, get_current_uid, get_group_by_name, get_user_by_name};

use crate::config::ProcessProbe;
use crate::unit;


#[derive(Deserialize, Debug)]
pub struct Unit {
    pub name: String,
    executable: String,
    arguments: Option<Vec<String>>,
    pub dependencies: Option<Vec<String>>,
    restart_policy: Option<unit::RestartPolicy>,
    user: Option<String>,
    group: Option<String>,
    enabled: Option<bool>,
    startup_probe: Option<ProcessProbe>,
    readiness_probe: Option<ProcessProbe>,
    liveness_probe: Option<ProcessProbe>,
}


impl Unit {
    pub fn build_ref(&self) -> unit::UnitRef {
        let restart_policy = match &self.restart_policy {
            Some(restart_policy) => restart_policy.clone(),
            None => unit::RestartPolicy::Always,
        };

        let enabled = match &self.enabled {
            Some(enabled) => *enabled,
            None => true,
        };

        let arguments = match &self.arguments {
            Some(arguments) => arguments.clone(),
            None => Vec::new(),
        };

        let startup_probe = match &self.startup_probe {
            Some(startup_probe) => Some(startup_probe.build_ref()),
            None => None,
        };

        let readiness_probe = match &self.readiness_probe {
            Some(readiness_probe) => Some(readiness_probe.build_ref()),
            None => None,
        };

        let liveness_probe = match &self.liveness_probe {
            Some(liveness_probe) => Some(liveness_probe.build_ref()),
            None => None,
        };

        return unit::Unit::new_ref(
            self.name.clone(),
            self.executable.clone(),
            arguments,
            restart_policy,
            self.determine_uid(),
            self.determine_gid(),
            enabled,
            startup_probe,
            readiness_probe,
            liveness_probe,
        );
    }

    /// If user is valid, return its uid
    /// Otherwise, return own uid
    fn determine_uid(&self) -> u32 {
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
    fn determine_gid(&self) -> u32 {
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
