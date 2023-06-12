use std::process::{Command, Child};
use std::os::unix::process::CommandExt;
use std::sync::{Arc, Mutex};


pub type UnitRef = Arc<Mutex<Unit>>;


#[derive(Debug, PartialEq)]
pub enum RestartPolicy {
    Always,
    Never,
}


pub struct Unit {
    name: String,
    executable: String,
    arguments: Vec<String>,
    dependencies: Vec<UnitRef>,
    restart_policy: RestartPolicy,
    uid: Option<u32>,
    gid: Option<u32>,
    enabled: bool,
    child: Option<Box<Child>>,
}


impl Unit {
    pub fn new(
        name: String,
        executable: String,
        arguments: Vec<String>,
        dependencies: Vec<UnitRef>,
        restart_policy: RestartPolicy,
        enabled: bool,
    ) -> Unit {
        Unit {
            name,
            executable,
            arguments,
            dependencies,
            restart_policy,
            uid: None,
            gid: None,
            enabled,
            child: None,
        }
    }

    pub fn new_ref(
        name: String,
        executable: String,
        arguments: Vec<String>,
        dependencies: Vec<UnitRef>,
        restart_policy: RestartPolicy,
        enabled: bool,
    ) -> UnitRef {
        Arc::new(Mutex::new(Unit::new(
            name,
            executable,
            arguments,
            dependencies,
            restart_policy,
            enabled,
        )))
    }

    pub fn name(&self) -> &String {
        return &self.name;
    }

    /// A unit is running if it has a child process
    pub fn is_running(&self) -> bool {
        return self.child.is_some();
    }

    /// Returns the Process ID (PID) of a child process, if it exists
    pub fn pid(&self) -> Option<u32> {
        match self.child {
            Some(ref child) => {
                return Some(child.id());
            }
            None => {
                return None;
            }
        }
    }

    /// Starts the child process
    pub fn start(&mut self) -> Result<bool, String> {
        // ignore if unit is running, i.e. child is not None
        if self.is_running() {
            return Err(format!("Unit {} is already running", self.name));
        }

        let mut command: Command = Command::new(&self.executable);
        command.args(&self.arguments);

        match self.uid {
            Some(uid) => {
                command.uid(uid);
            }
            None => {}
        }

        match self.gid {
            Some(gid) => {
                command.gid(gid);
            }
            None => {}
        }

        let child = command.spawn();

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
    fn can_start(&self) -> Result<bool, String> {
        if !self.enabled {
            return Err(format!("Unit {} is not enabled", self.name));
        }

        for dependency in &self.dependencies {
            let unit = dependency.lock().unwrap();
            if !unit.is_running() {
                return Err(format!("Cannot start unit {} because its dependency unit {} is NOT running",
                                   self.name, unit.name));
            }
        }

        return Ok(true);
    }

    /// A unit is allowed to start if it is enabled and all dependencies are stopped
    fn can_stop(&self) -> Result<bool, String> {
        if !self.enabled {
            return Err(format!("Cannot stop unit {} because it is disabled", self.name));
        }

        for dependency in &self.dependencies {
            let unit = dependency.lock().unwrap();
            if unit.is_running() {
                return Err(format!("Cannot stop unit {} because its dependency unit {} is running",
                                   self.name, unit.name));
            }
        }

        return Ok(true);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn build_unit() -> Unit {
        return Unit::new(
            String::from("test"),
            String::from("sleep"),
            vec![String::from("1")],
            vec![],
            RestartPolicy::Never,
            true,
        );
    }

    fn build_unitrefs() -> (UnitRef, UnitRef) {
        let unit1 = Unit::new_ref(
            String::from("test1"),
            String::from("ls"),
            vec![],
            vec![],
            RestartPolicy::Always,
            true,
        );

        let unit2 = Unit::new_ref(
            String::from("test2"),
            String::from("ls"),
            vec![],
            vec![unit1.clone()],
            RestartPolicy::Never,
            true,
        );

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
        let unit = build_unit();

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
        assert_eq!(unit2.lock().unwrap().can_start().unwrap(), true);
    }

    #[test]
    fn cannot_start_when_dependent_unit_is_not_running() {
        let (unit1, unit2) = build_unitrefs();

        assert_eq!(unit1.lock().unwrap().is_running(), false);
        assert!(unit2.lock().unwrap().can_start().is_err());
    }

    #[test]
    fn can_stop_when_dependent_unit_is_not_running() {
        let (unit1, unit2) = build_unitrefs();

        assert_eq!(unit1.lock().unwrap().is_running(), false);
        assert_eq!(unit2.lock().unwrap().can_stop().unwrap(), true);
    }

    #[test]
    fn cannot_stop_when_dependent_unit_is_running() {
        let (unit1, unit2) = build_unitrefs();

        unit1.lock().unwrap().start().unwrap();
        assert_eq!(unit1.lock().unwrap().is_running(), true);
        assert!(unit2.lock().unwrap().can_stop().is_err());
    }
}
