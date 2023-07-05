use std::process::{Child, Command, Stdio};
use std::os::unix::process::CommandExt;
use std::sync::{Arc, Mutex};
use log::{debug, warn, trace};

use crate::unit::{RestartPolicy, ProcessProbe, ProcessProbeRef, LivenessProbeRef, ProbeState};


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
    liveness_probe: Option<LivenessProbeRef>,
    process_probe: Option<ProcessProbeRef>,
    child: Option<Box<Child>>,
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
        liveness_probe: Option<LivenessProbeRef>,
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
            process_probe: None,
            child: None,
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
        liveness_probe: Option<LivenessProbeRef>,
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
        self.name.clone()
    }

    pub fn get_executable(&self) -> String {
        self.executable.clone()
    }

    pub fn get_arguments(&self) -> Vec<String> {
        self.arguments.clone()
    }

    pub fn add_dependency(&mut self, unit: UnitRef) {
        self.dependencies.push(unit);
    }

    pub fn get_dependencies(&self) -> Vec<UnitRef> {
        self.dependencies.clone()
    }

    pub fn get_restart_policy(&self) -> RestartPolicy {
        self.restart_policy.clone()
    }

    pub fn get_uid(&self) -> u32 {
        self.uid
    }

    pub fn get_gid(&self) -> u32 {
        self.gid
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_pid(&self) -> Option<u32> {
        return match self.child {
            Some(ref child) => Some(child.as_ref().id()),
            None => None,
        };
    }

    pub fn get_process_probe_state(&self) -> ProbeState {
        return match self.process_probe {
            Some(ref process_probe) => {
                match process_probe.lock() {
                    Ok(process_probe) => {
                        let probe_state = process_probe.get_state();
                        trace!("Unit {} process probe state: {:?}", self.name, probe_state);
                        probe_state
                    }
                    Err(error) => {
                        warn!("Unit {} failed to get process probe state: {}", self.name, error);
                        ProbeState::Undefined
                    }
                }
            }
            None => {
                trace!("Unit {} does not have a process probe yet", self.name);
                ProbeState::Undefined
            }
        };
    }

    pub fn get_liveness_probe_state(&self) -> ProbeState {
        return match self.liveness_probe {
            Some(ref liveness_probe) => {
                match liveness_probe.lock() {
                    Ok(liveness_probe) => {
                        liveness_probe.get_state()
                    }
                    Err(error) => {
                        warn!("Unit {} failed to get liveness probe state: {}", self.name, error);
                        ProbeState::Undefined
                    }
                }
            }
            None => {
                ProbeState::Undefined
            }
        };
    }

    pub fn is_running(&self) -> bool {
        self.get_process_probe_state() == ProbeState::Alive
    }

    /// Starts the child process
    pub fn start(&mut self) -> Result<bool, String> {
        if !self.can_start() {
            return Err(format!("Cannot start unit {}", self.name));
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
                self.start_probes();
                Ok(true)
            }
            Err(error) => {
                self.child = None;
                Err(format!("Unit {} failed to start: {}", self.name, error))
            }
        }
    }

    /// Stops the child process
    pub fn stop(&mut self) -> Result<bool, String> {
        if !self.can_stop() {
            return Err(format!("Cannot stop unit {}", self.name));
        }

        match self.child {
            Some(ref mut child) => {
                match child.kill() {
                    Ok(_) => {
                        debug!("Unit {} was stopped", self.name);
                        self.stop_probes();
                        self.cleanup_process_handle();
                        Ok(true)
                    }
                    Err(error) => {
                        Err(format!("Unit {} failed to stop: {}", self.name, error))
                    }
                }
            }
            None => {
                warn!("Cannot stop unit {} because it is NOT running", self.name);
                Ok(false)
            }
        }
    }

    pub fn restart(&mut self) -> Result<bool, String> {
        debug!("Restarting unit {}", self.name);
        match self.stop() {
            Ok(_) => {
                match self.start() {
                    Ok(_) => {
                        Ok(true)
                    }
                    Err(error) => {
                        Err(error)
                    }
                }
            }
            Err(error) => {
                Err(error)
            }
        }
    }

    /// A unit is allowed to start if it is enabled and all dependencies are running
    fn can_start(&mut self) -> bool {
        if !self.enabled {
            warn!("Unit {} is not enabled", self.name);
            return false;
        }

        // ignore if unit is running
        if self.is_running() {
            warn!("Unit {} is already running", self.name);
            return false;
        }

        for dependency in &self.dependencies {
            match dependency.lock() {
                Ok(unit) => {
                    if !unit.is_running() {
                        // dependency is not running, so we cannot start
                        warn!("Unit {} cannot start because dependency {} is not running", self.name, unit.name);
                        return false;
                    }
                }
                Err(error) => {
                    warn!("Unit {} failed to lock dependency: {}", self.name, error);
                    return false;
                }
            }
        }

        return true;
    }

    /// A unit is allowed to stop if it is running
    fn can_stop(&self) -> bool {
        if !self.is_enabled() {
            warn!("Unit {} is not enabled", self.name);
            return false;
        }

        // ignore if unit is stopped
        if !self.is_running() {
            warn!("Unit {} is already stopped", self.name);
            return false;
        }

        return true;
    }

    fn start_probes(&mut self) {
        // Start process probe
        match self.get_pid() {
            Some(pid) => {
                let process_probe = ProcessProbe::new(
                    self.get_name(),
                    pid,
                    5,
                );

                process_probe.run();
                self.process_probe = Some(Arc::new(Mutex::new(process_probe)));
            }
            None => {
                warn!("Cannot start process probe for unit {}", self.name);
            }
        }

        // Start liveness probe
        match self.liveness_probe {
            Some(ref liveness_probe) => {
                match liveness_probe.lock() {
                    Ok(liveness_probe) => {
                        liveness_probe.run();
                    }
                    Err(error) => {
                        warn!("Unit {} failed to start liveness probe: {}", self.name, error);
                    }
                }
            }
            None => {}
        }
    }

    fn stop_probes(&mut self) {
        // Stop process probe
        match self.process_probe {
            Some(ref process_probe) => {
                match process_probe.lock() {
                    Ok(mut process_probe) => {
                        process_probe.request_stop();
                    }
                    Err(error) => {
                        warn!("Unit {} failed to stop process probe: {}", self.name, error);
                    }
                }
            }
            None => {}
        }

        // Stop liveness probe
        match self.liveness_probe {
            Some(ref liveness_probe) => {
                match liveness_probe.lock() {
                    Ok(mut liveness_probe) => {
                        liveness_probe.request_stop();
                    }
                    Err(error) => {
                        warn!("Unit {} failed to stop liveness probe: {}", self.name, error);
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

        assert_eq!(unit.is_running(), false);
    }

    #[test]
    fn is_running_returns_correct_values_after_start() {
        let mut unit = build_unit();

        assert_eq!(unit.is_running(), false);
        unit.start().unwrap();
        assert_eq!(unit.is_running(), true);
    }

    #[test]
    fn is_running_returns_correct_values_after_stop() {
        let mut unit = build_unit();

        assert_eq!(unit.is_running(), false);
        unit.start().unwrap();
        assert_eq!(unit.is_running(), true);
        unit.stop().unwrap();
        assert_eq!(unit.is_running(), false);
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
        assert_eq!(unit1.lock().unwrap().is_running(), true);
        assert_eq!(unit2.lock().unwrap().can_start(), true);
    }

    #[test]
    fn cannot_start_when_dependent_unit_is_not_running() {
        let (unit1, unit2) = build_unitrefs();

        assert_eq!(unit1.lock().unwrap().is_running(), false);
        assert_eq!(unit2.lock().unwrap().can_start(), false);
    }

    #[test]
    fn can_stop_when_dependent_unit_is_not_running() {
        let (unit1, unit2) = build_unitrefs();

        unit1.lock().unwrap().start().unwrap();
        unit2.lock().unwrap().start().unwrap();
        unit1.lock().unwrap().stop().unwrap();

        assert_eq!(unit1.lock().unwrap().is_running(), false);
        assert_eq!(unit2.lock().unwrap().can_stop(), true);
    }

    #[test]
    fn cannot_stop_when_dependent_unit_is_running() {
        let (unit1, unit2) = build_unitrefs();

        unit1.lock().unwrap().start().unwrap();
        assert_eq!(unit1.lock().unwrap().is_running(), true);
        assert_eq!(unit2.lock().unwrap().can_stop(), false);
    }
}
