use std::process::{Child, Command};
use std::os::unix::process::CommandExt;
use std::sync::{Arc, Mutex};
use sysinfo::{Pid, PidExt, ProcessRefreshKind, System, SystemExt};

use crate::unit::{RestartPolicy, ProcessProbeRef, ProbeState};


pub type UnitRef = Arc<Mutex<Unit>>;


#[derive(Debug)]
pub struct Unit {
    name: String,
    executable: String,
    arguments: Vec<String>,
    dependencies: Vec<UnitRef>,
    restart_policy: RestartPolicy,
    uid: u32,
    gid: u32,
    enabled: bool,
    startup_probe: Option<ProcessProbeRef>,
    readiness_probe: Option<ProcessProbeRef>,
    liveness_probe: Option<ProcessProbeRef>,
    child: Option<Box<Child>>,
    system_info: System,
    probe_state: ProbeState,
}


impl Unit {
    pub fn new(
        name: String,
        executable: String,
        arguments: Vec<String>,
        restart_policy: RestartPolicy,
        uid: u32,
        gid: u32,
        enabled: bool,
        startup_probe: Option<ProcessProbeRef>,
        readiness_probe: Option<ProcessProbeRef>,
        liveness_probe: Option<ProcessProbeRef>,
    ) -> Unit {
        Unit {
            name,
            executable,
            arguments,
            dependencies: Vec::new(),
            restart_policy,
            uid,
            gid,
            enabled,
            startup_probe,
            readiness_probe,
            liveness_probe,
            child: None,
            system_info: System::new(),
            probe_state: ProbeState::Unknown,
        }
    }

    pub fn new_ref(
        name: String,
        executable: String,
        arguments: Vec<String>,
        restart_policy: RestartPolicy,
        uid: u32,
        gid: u32,
        enabled: bool,
        startup_probe: Option<ProcessProbeRef>,
        readiness_probe: Option<ProcessProbeRef>,
        liveness_probe: Option<ProcessProbeRef>,
    ) -> UnitRef {
        Arc::new(Mutex::new(Unit::new(
            name,
            executable,
            arguments,
            restart_policy,
            uid,
            gid,
            enabled,
            startup_probe,
            readiness_probe,
            liveness_probe,
        )))
    }

    pub fn add_dependency(&mut self, unit: UnitRef) {
        self.dependencies.push(unit);
    }

    pub fn dependencies(&self) -> &Vec<UnitRef> {
        return &self.dependencies;
    }

    pub fn name(&self) -> &String {
        return &self.name;
    }

    pub fn restart_policy(&self) -> &RestartPolicy {
        return &self.restart_policy;
    }

    /// A unit is running if it has a child process
    pub fn test_running(&mut self) -> bool {
        return match self.child {
            Some(ref child) => {
                // Check if child process is still alive
                let pid = Pid::from_u32(child.id());
                let refresh_kind = ProcessRefreshKind::new();
                let process_exists = self.system_info.refresh_process_specifics(pid, refresh_kind);

                if process_exists {
                    true
                } else {
                    self.child = None;
                    false
                }
            }
            None => {
                false
            }
        }
    }

    /// Returns the Process ID (PID) of a child process, if it exists
    pub fn pid(&self) -> Option<u32> {
        return match self.child {
            Some(ref child) => {
                Some(child.id())
            }
            None => {
                None
            }
        }
    }

    /// Starts the child process
    pub fn start(&mut self) -> Result<bool, String> {
        let (can_start, reason) = self.can_start();

        if !can_start {
            return Err(format!("Cannot start unit {}: {}", self.name, reason));
        }

        let child = Command::new(&self.executable)
            .args(&self.arguments)
            .uid(self.uid)
            .gid(self.gid)
            .spawn();

        match child {
            Ok(child) => {
                self.child = Some(Box::new(child));
            }
            Err(error) => {
                self.child = None;
                return Err(format!("Unit {} failed to start: {}", self.name, error));
            }
        }

        return Ok(true);
    }

    /// Stops the child process
    pub fn stop(&mut self) -> Result<bool, String> {
        let (can_stop, reason) = self.can_stop();

        if !can_stop {
            return Err(format!("Cannot stop unit {}: {}", self.name, reason));
        }

        match self.child {
            Some(ref mut child) => {
                match child.kill() {
                    Ok(_) => {
                        self.child = None;
                    }
                    Err(error) => {
                        return Err(format!("Unit {} failed to stop: {}", self.name, error));
                    }
                }
            }
            None => {
                return Err(format!("Cannot stop unit {} because it is NOT running", self.name));
            }
        }

        return Ok(true);
    }

    /// A unit is allowed to start if it is enabled and all dependencies are running
    fn can_start(&mut self) -> (bool, String) {
        if !self.enabled {
            return (false, String::from("Unit is not enabled"));
        }

        // ignore if unit is running, i.e. child is not None
        if self.test_running() {
            return (false, String::from("Unit is already running"));
        }

        for dependency in &self.dependencies {
            let mut unit = dependency.lock().unwrap();
            if !unit.test_running() {
                return (false, format!("Unit depends on {} but it is not running", unit.name));
            }
        }

        return (true, String::from(""));
    }

    /// A unit is allowed to start if it is enabled and all dependencies are stopped
    fn can_stop(&mut self) -> (bool, String) {
        if !self.enabled {
            return (false, String::from("Unit is not enabled"));
        }

        // ignore if unit is stopped, i.e. child is None
        if !self.test_running() {
            return (false, String::from("Unit is already stopped"));
        }

        for dependency in &self.dependencies {
            let mut unit = dependency.lock().unwrap();
            if unit.test_running() {
                return (false, format!("Unit depends on {} but it is still running", unit.name));
            }
        }

        return (true, String::from(""));
    }

    /// Execute .probe() on all probes and return true if all probes are successful
    pub fn probe_all(&mut self) -> Result<bool, String> {
        let mut success = true;

        if self.probe_state == ProbeState::Unknown {
            match self.startup_probe {
                Some(ref mut probe) => {
                    let probe_success_result = probe.lock().unwrap().probe();
                    match probe_success_result {
                        Ok(probe_success) => {
                            success = success && probe_success;
                            self.probe_state = ProbeState::Startup;
                        }
                        Err(error) => {
                            return Err(format!("Failed to probe unit {} for startup: {}", self.name, error));
                        }
                    }
                }
                None => {}
            }
        }

        if self.probe_state == ProbeState::Startup {
            match self.readiness_probe {
                Some(ref mut probe) => {
                    let probe_success_result = probe.lock().unwrap().probe();
                    match probe_success_result {
                        Ok(probe_success) => {
                            success = success && probe_success;
                            self.probe_state = ProbeState::Ready;
                        }
                        Err(error) => {
                            return Err(format!("Failed to probe unit {} for readiness: {}", self.name, error));
                        }
                    }
                }
                None => {}
            }
        }

        if self.probe_state == ProbeState::Ready {
            match self.liveness_probe {
                Some(ref mut probe) => {
                    let probe_success_result = probe.lock().unwrap().probe();
                    match probe_success_result {
                        Ok(probe_success) => {
                            success = success && probe_success;
                            self.probe_state = ProbeState::Alive;
                        }
                        Err(error) => {
                            return Err(format!("Failed to probe unit {} for liveness: {}", self.name, error));
                        }
                    }
                }
                None => {}
            }
        }

        return Ok(success);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use users::{get_current_gid, get_current_uid};

    fn build_unit() -> Unit {
        return Unit::new(
            String::from("test"),
            String::from("sleep"),
            vec![String::from("1")],
            RestartPolicy::Never,
            get_current_uid(),
            get_current_gid(),
            true,
            None,
            None,
            None,
        );
    }

    fn build_unitrefs() -> (UnitRef, UnitRef) {
        let unit1 = Unit::new_ref(
            String::from("test1"),
            String::from("ls"),
            vec![],
            RestartPolicy::Always,
            get_current_uid(),
            get_current_gid(),
            true,
            None,
            None,
            None,
        );

        let unit2 = Unit::new_ref(
            String::from("test2"),
            String::from("ls"),
            vec![],
            RestartPolicy::Never,
            get_current_uid(),
            get_current_gid(),
            true,
            None,
            None,
            None,
        );

        unit2.lock().unwrap().add_dependency(unit1.clone());

        return (unit1, unit2);
    }

    #[test]
    fn new_returns_new_unit() {
        let unit = build_unit();

        assert_eq!(unit.name, "test");
        assert_eq!(unit.executable, "sleep");
        assert_eq!(unit.arguments, vec!["1"]);
        assert_eq!(unit.dependencies.is_empty(), true);
        assert_eq!(unit.restart_policy, RestartPolicy::Never);
        assert_eq!(unit.enabled, true);
    }

    #[test]
    fn new_returns_correct_name() {
        let unit = build_unit();

        assert_eq!(unit.name(), "test");
    }

    #[test]
    fn pid_returns_not_none() {
        let mut unit = build_unit();

        unit.start().unwrap();
        assert_ne!(unit.pid(), None);
    }

    #[test]
    fn is_running_returns_correct_values_at_init() {
        let mut unit = build_unit();

        assert_eq!(unit.test_running(), false);
    }

    #[test]
    fn is_running_returns_correct_values_after_start() {
        let mut unit = build_unit();

        assert_eq!(unit.test_running(), false);
        unit.start().unwrap();
        assert_eq!(unit.test_running(), true);
    }

    #[test]
    fn is_running_returns_correct_values_after_stop() {
        let mut unit = build_unit();

        assert_eq!(unit.test_running(), false);
        unit.start().unwrap();
        assert_eq!(unit.test_running(), true);
        unit.stop().unwrap();
        assert_eq!(unit.test_running(), false);
    }

    #[test]
    fn cannot_start_if_already_started() {
        let mut unit = build_unit();

        unit.start().unwrap();
        assert!(unit.start().is_err());
    }

    #[test]
    fn cannot_stop_if_already_stopped() {
        let mut unit = build_unit();

        assert!(unit.stop().is_err());
    }

    #[test]
    fn can_start_when_dependent_unit_is_running() {
        let (unit1, unit2) = build_unitrefs();

        unit1.lock().unwrap().start().unwrap();
        assert_eq!(unit1.lock().unwrap().test_running(), true);
        assert_eq!(unit2.lock().unwrap().can_start().0, true);
    }

    #[test]
    fn cannot_start_when_dependent_unit_is_not_running() {
        let (unit1, unit2) = build_unitrefs();

        assert_eq!(unit1.lock().unwrap().test_running(), false);
        assert_eq!(unit2.lock().unwrap().can_start().0, false);
    }

    #[test]
    fn can_stop_when_dependent_unit_is_not_running() {
        let (unit1, unit2) = build_unitrefs();

        unit1.lock().unwrap().start().unwrap();
        unit2.lock().unwrap().start().unwrap();
        unit1.lock().unwrap().stop().unwrap();

        assert_eq!(unit1.lock().unwrap().test_running(), false);
        assert_eq!(unit2.lock().unwrap().can_stop().0, true);
    }

    #[test]
    fn cannot_stop_when_dependent_unit_is_running() {
        let (unit1, unit2) = build_unitrefs();

        unit1.lock().unwrap().start().unwrap();
        assert_eq!(unit1.lock().unwrap().test_running(), true);
        assert_eq!(unit2.lock().unwrap().can_stop().0, false);
    }
}
