use std::sync::{Arc, Mutex};
use log::{debug, warn};

use crate::unit::{RestartPolicy, ProcessProbe, LivenessProbe, ProbeState, Process, ProbeManager};


pub type UnitRef = Arc<Mutex<Unit>>;


#[derive(Debug)]
pub struct Unit {
    name: String,
    dependencies: Vec<UnitRef>,
    restart_policy: RestartPolicy,
    enabled: bool,
    process: Process,
    probe_manager: ProbeManager,
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
    ) -> Unit {
        // init process
        let process = Process::new(
            executable.clone(),
            arguments.clone(),
            uid,
            gid,
        );

        Unit {
            name: name.clone(),
            dependencies: Vec::new(),
            restart_policy,
            enabled,
            process,
            probe_manager: ProbeManager::new(name.clone()),
        }
    }

    pub fn set_liveness_probe(&mut self, probe: LivenessProbe) {
        self.probe_manager.set_liveness_probe(probe);
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_executable(&self) -> String {
        self.process.get_executable()
    }

    pub fn get_arguments(&self) -> Vec<String> {
        self.process.get_arguments()
    }

    pub fn add_dependency(&mut self, unit: UnitRef) {
        self.dependencies.push(unit);
    }

    pub fn get_restart_policy(&self) -> RestartPolicy {
        self.restart_policy.clone()
    }

    pub fn get_uid(&self) -> u32 {
        self.process.get_uid()
    }

    pub fn get_gid(&self) -> u32 {
        self.process.get_gid()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_pid(&self) -> Option<u32> {
        self.process.get_pid()
    }

    pub fn get_process_probe_state(&self) -> ProbeState {
        self.probe_manager.get_process_probe_state()
    }

    pub fn get_liveness_probe_state(&self) -> ProbeState {
        self.probe_manager.get_liveness_probe_state()
    }

    /// Checks if the unit is running.
    /// Called periodically by unit manager.
    pub fn is_running(&mut self) -> bool {
        if self.process.is_running() {
            return true;
        }

        // Process might be still running, check whether its pid is still valid
        return self.get_process_probe_state() == ProbeState::Alive
    }

    /// Starts the child process
    /// This does not start the probes!
    pub fn start(&mut self) -> Result<bool, String> {
        if !self.can_start() {
            return Err(format!("Cannot start unit {}", self.name));
        }

        debug!("Starting unit {}", self.name);

        match self.start_dependencies() {
            Ok(_) => {}
            Err(error) => {
                return Err(format!("Unit {} failed to start dependencies: {}", self.name, error));
            }
        }

        match self.process.start() {
            Ok(_) => {
                debug!("Unit {} was started", self.name);
                self.init_process_probe();
                Ok(true)
            }
            Err(error) => {
                Err(format!("Unit {} failed to start: {}", self.name, error))
            }
        }
    }

    fn init_process_probe(&mut self) {
        match self.process.get_pid() {
            Some(pid) => {
                let process_probe = ProcessProbe::new(
                    self.name.clone(),
                    pid,
                    5,
                );
                self.probe_manager.set_process_probe(process_probe);
            },
            None => {}
        }
    }

    fn start_dependencies(&mut self) -> Result<bool, String> {
        for dependency in &self.dependencies {
            match dependency.try_lock() {
                Ok(mut unit) => {
                    if !unit.is_running() {
                        unit.start()?;
                    }
                },
                Err(error) => {
                    return Err(format!("Unit {} failed to start: {}", self.name, error));
                }
            }
        }

        Ok(true)
    }

    /// Stops the child process
    pub fn stop(&mut self) -> Result<bool, String> {
        debug!("Stopping unit {}", self.name);

        // stop probes first to prevent process from being restarted during stopping
        self.stop_probes();

        match self.process.stop() {
            Ok(_) => {
                debug!("Unit {} was stopped", self.name);
                Ok(true)
            }
            Err(error) => {
                self.start_probes(); // restart probes since stopping process failed
                Err(format!("Unit {} failed to stop: {}", self.name, error))
            }
        }
    }

    pub fn restart(&mut self) -> Result<bool, String> {
        debug!("Restarting unit {}", self.name);

        if self.is_running() {
            self.stop()?;
        }

        self.start()
    }

    /// A unit is allowed to start if it is enabled and all dependencies are running
    fn can_start(&mut self) -> bool {
        if !self.enabled {
            warn!("Unit {} is not enabled", self.name);
            return false;
        }

        // ignore if unit is running
        if self.is_running() {
            debug!("Unit {} is already running", self.name);
            return false;
        }

        return true;
    }

    fn are_dependencies_running(&mut self) -> bool {
        for dependency in &self.dependencies {
            match dependency.try_lock() {
                Ok(mut unit) => {
                    if !unit.is_running() {
                        return false;
                    }
                },
                Err(error) => {
                    warn!("Unit {} failed to acquire lock: {}", self.name, error);
                    return false;
                }
            }
        }

        return true;
    }

    /// A unit is allowed to stop if it is enabled and it is running regardless of its dependencies
    fn can_stop(&mut self) -> bool {
        if !self.is_enabled() {
            warn!("Unit {} is not enabled", self.name);
            return false;
        }

        // ignore if unit is stopped
        if !self.is_running() {
            debug!("Unit {} is already stopped", self.name);
            return false;
        }

        return true;
    }

    pub fn start_probes(&mut self) {
        if !self.is_running() {
            warn!("Cannot start probes for unit {} because it is not running", self.name);
            return;
        }

        debug!("Starting probes for unit {}", self.name);
        self.probe_manager.start_probes();
    }

    fn stop_probes(&mut self) {
        debug!("Stopping probes for unit {}", self.name);
        self.probe_manager.stop_probes();
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
