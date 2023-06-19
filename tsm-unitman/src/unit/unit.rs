use std::process::{Child, Command, Stdio};
use std::os::unix::process::CommandExt;
use std::sync::{Arc, Mutex};
use sysinfo::{Pid, PidExt, ProcessRefreshKind, System, SystemExt};
use log::{debug, warn, trace};

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
            liveness_probe,
            child: None,
            system_info: System::new(),
            probe_state: ProbeState::Undefined,
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
            liveness_probe,
        )))
    }

    pub fn get_name(&self) -> String {
        return self.name.clone();
    }

    pub fn get_executable(&self) -> String {
        return self.executable.clone();
    }

    pub fn get_arguments(&self) -> Vec<String> {
        return self.arguments.clone();
    }

    pub fn add_dependency(&mut self, unit: UnitRef) {
        self.dependencies.push(unit);
    }

    pub fn get_dependencies(&self) -> &Vec<UnitRef> {
        return &self.dependencies;
    }

    pub fn get_restart_policy(&self) -> RestartPolicy {
        return self.restart_policy.clone();
    }

    pub fn get_uid(&self) -> u32 {
        return self.uid;
    }

    pub fn get_gid(&self) -> u32 {
        return self.gid;
    }

    pub fn is_enabled(&self) -> bool {
        return self.enabled;
    }

    pub fn get_liveness_probe(&self) -> Option<ProcessProbeRef> {
        return self.liveness_probe.clone();
    }

    pub fn get_probe_state(&self) -> ProbeState {
        return self.probe_state.clone();
    }

    /// A unit is running if its pid exists
    pub fn test_running(&mut self) -> bool {
        return match self.child {
            Some(ref _child) => {
                match self.get_pid() {
                    Some(_pid) => {
                        true
                    }
                    None => {
                        self.cleanup_process_handle();
                        self.cleanup_liveness_probe_prevent_unit_restart();
                        false
                    }
                }
            }
            None => {
                false
            }
        }
    }

    /// Returns the Process ID (PID) of a child process, if it exists
    /// We also check whether the process is still alive.
    pub fn get_pid(&mut self) -> Option<u32> {
        return match self.child {
            Some(ref mut child) => {
                // Check whether we have exit code, which means that the process was exited
                match child.try_wait() {
                    Ok(Some(exit_status)) => {
                        // Process is not running anymore
                        trace!("Unit {} exited with code {}", self.name, exit_status);
                        None
                    }
                    Ok(None) | Err(_) => {
                        // Case child does not have an exit code or asking for exit code failed
                        // Process is maybe still running. Check whether pid still exists.
                        let pid = Pid::from_u32(child.id());
                        let refresh_kind = ProcessRefreshKind::new();
                        let process_exists = self.system_info.refresh_process_specifics(pid, refresh_kind);

                        if process_exists {
                            trace!("Unit {} is running with pid {}", self.name, pid);
                            Some(child.id())
                        } else {
                            trace!("Unit {} is NOT running, pid {} does not exist anymore", self.name, pid);
                            None
                        }
                    }
                }
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
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .uid(self.uid)
            .gid(self.gid)
            .spawn();

        match child {
            Ok(child) => {
                debug!("Unit {} was started", self.name);
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
                        debug!("Unit {} was stopped", self.name);
                        self.cleanup_process_handle();
                        self.cleanup_liveness_probe_prevent_unit_restart();
                        Ok(true)
                    }
                    Err(error) => {
                        Err(format!("Unit {} failed to stop: {}", self.name, error))
                    }
                }
            }
            None => {
                Err(format!("Cannot stop unit {} because it is NOT running", self.name))
            }
        }
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

    pub fn liveness_probe(&mut self) {
        match self.liveness_probe {
            Some(ref mut probe) => {
                trace!("Unit {} probing", self.name);

                let probe_success_result = probe.lock().unwrap().probe();
                match probe_success_result {
                    Ok(probe_success) => { // true if probe succeeded, false if not time to probe yet
                        if probe_success {
                            debug!("Unit {} probe succeeded. Setting probe state to Alive.", self.name);
                            self.probe_state = ProbeState::Alive;
                        }
                    }
                    Err(error) => {
                        warn!("Unit {} probe failed. Process does not exist anymore. Setting probe state to Dead: {}", self.name, error);
                        self.cleanup_process_handle();
                        self.cleanup_liveness_probe_prevent_unit_restart(); // sets probe_state to Dead
                    }
                }
            }
            None => {}
        }
    }

    fn cleanup_process_handle(&mut self) {
        match self.child {
            Some(ref mut child) => {
                match child.stdout.take() {
                    Some(stdout) => {
                        drop(stdout);
                    }
                    None => {}
                }

                match child.stderr.take() {
                    Some(stderr) => {
                        drop(stderr);
                    }
                    None => {}
                }

                self.child = None;
            }
            None => {}
        }
    }

    fn cleanup_liveness_probe_prevent_unit_restart(&mut self) {
        match self.liveness_probe {
            Some(ref mut _probe) => {
                if self.restart_policy == RestartPolicy::Never {
                    debug!("Restart policy of unit {} was configured to never. Stopping probe and setting probe state to Dead.", self.name);
                    self.liveness_probe = None;
                    self.probe_state = ProbeState::Dead;
                }
            }
            None => {}
        }
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

        assert_eq!(unit.get_name(), "test");
    }

    #[test]
    fn pid_returns_not_none() {
        let mut unit = build_unit();

        unit.start().unwrap();
        assert_ne!(unit.get_pid(), None);
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
