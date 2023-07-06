use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use log::{debug, error, warn};

use crate::unit::{RestartPolicy, UnitRef};


pub type UnitManagerRef = Arc<Mutex<UnitManager>>;


#[derive(Clone, Debug)]
pub struct UnitManager {
    units: Vec<UnitRef>,
    stop_requested: Arc<Mutex<bool>>,
}


impl UnitManager {
    pub fn new() -> UnitManager {
        UnitManager {
            units: Vec::new(),
            stop_requested: Arc::new(Mutex::new(false)),
        }
    }

    pub fn new_ref() -> UnitManagerRef {
        Arc::new(Mutex::new(UnitManager::new()))
    }

    pub fn add_unit(&mut self, unit: UnitRef) {
        match unit.try_lock() {
            Ok(unit_unlocked) => {
                debug!("Adding unit {:?}", unit_unlocked);
                self.units.push(unit.clone());
            }
            Err(e) => {
                error!("Failed to lock unit: {}", e);
            }
        }
    }

    pub fn get_units(&self) -> &Vec<UnitRef> {
        &self.units
    }

    /// Iterate over all units and try to start them
    /// Note that we need to call this function several times until all dependencies are started
    fn start_units(&mut self) {
        self.reset_stop_request();

        // TODO: Max retries
        while !self.all_units_running() {
            for unit in &self.units {
                match unit.try_lock() {
                    Ok(mut unit) => {
                        if !unit.is_running() {
                            match unit.start() {
                                Ok(_) => debug!("Started unit {}", unit.get_name()),
                                Err(e) => warn!("Error starting unit {}: {}", unit.get_name(), e),
                            }
                        }
                    }
                    Err(e) => error!("Error acquiring lock while starting unit: {}", e),
                }
            }

            thread::sleep(Duration::from_secs(1));
        }

        // start probes
        // Probes are not started inside the loop above because if a unit is stopped
        // inside the loop, it will be started again by the loop
        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => unit.start_probes(),
                Err(e) => error!("Error acquiring lock while starting unit probes: {}", e),
            }
        }
    }

    /// Iterate over all units and try to stop them
    /// Units will be stopped regardless of their dependencies
    fn stop_units(&mut self) {
        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => {
                    // stopping unit will automatically stop its probes and cleanup its resources
                    match unit.stop() {
                        Ok(_) => debug!("Stopped unit {}", unit.get_name()),
                        Err(e) => warn!("Error stopping unit {}: {}", unit.get_name(), e),
                    }
                }
                Err(e) => error!("Error acquiring lock while stopping unit: {}", e),
            }
        }

        thread::sleep(Duration::from_secs(1));
    }

    /// Returns true if all units are running
    /// Returns false if at least one unit is not running
    fn all_units_running(&self) -> bool {
        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => {
                    if !unit.is_running() {
                        return false;
                    }
                }
                Err(e) => {
                    warn!("Error acquiring lock while checking if all units are running: {}", e);
                    return false;
                }
            }
        }

        return true;
    }

    /// Returns true if all units are stopped
    /// Returns false if at least one unit is not stopped
    fn all_units_stopped(&self) -> bool {
        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => {
                    if unit.is_running() {
                        return false;
                    }
                }
                Err(e) => {
                    warn!("Error acquiring lock while checking if all units are stopped: {}", e);
                    return false;
                }
            }
        }

        return true;
    }

    fn stop_requested(&self) -> bool {
        return match self.stop_requested.try_lock() {
            Ok(stop_requested) => *stop_requested,
            Err(e) => {
                error!("Failed to lock stop_requested: {}", e);
                false
            }
        };
    }

    /// Set stop_requested flag to true
    pub fn request_stop(&mut self) {
        match self.stop_requested.try_lock() {
            Ok(mut stop_requested) => {
                *stop_requested = true;
            },
            Err(e) => error!("Failed to lock stop_requested: {}", e),
        };
    }

    /// reset stop_requested to false
    pub fn reset_stop_request(&mut self) {
        match self.stop_requested.try_lock() {
            Ok(mut stop_requested) => *stop_requested = false,
            Err(e) => error!("Failed to lock stop_requested: {}", e),
        };
    }

    pub fn run(&self) -> JoinHandle<()> {
        let mut self_clone = self.clone();
        return thread::spawn(move || self_clone.run_loop());
    }

    pub fn run_loop(&mut self) {
        debug!("Starting units");
        self.start_units();

        debug!("Monitoring units");
        loop {
            if self.stop_requested() {
                debug!("Stop requested");
                break;
            }

            self.monitor();
            thread::sleep(Duration::from_secs(1));
        }

        debug!("Shutting down units");
        self.stop_units();
        self.reset_stop_request();
    }

    fn monitor(&self) {
        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => {
                    let is_running = unit.is_running();

                    if !is_running {
                        unit.stop(); // force cleanup resources
                    }

                    if !is_running && unit.get_restart_policy() == RestartPolicy::Always {
                        debug!("Unit {} is not running, restarting because restart policy was set to Always.", unit.get_name());
                        match unit.restart() {
                            Ok(_) => debug!("Unit {} restarted", unit.get_name()),
                            Err(e) => warn!("Error restarting unit {}: {}", unit.get_name(), e),
                        }
                    }
                }
                Err(e) => {
                    error!("Error acquiring lock while watching units: {}", e);
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use users::{get_current_gid, get_current_uid};
    use crate::unit::restart_policy::RestartPolicy;
    use crate::unit::unit::Unit;

    fn build_unitrefs() -> (UnitRef, UnitRef) {
        let unit1 = Unit::new_ref(
            String::from("test1"),
            String::from("sleep"),
            vec![String::from("1")],
            RestartPolicy::Always,
            get_current_uid(),
            get_current_gid(),
            true,
            None,
        );

        let unit2 = Unit::new_ref(
            String::from("test2"),
            String::from("sleep"),
            vec![String::from("1")],
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
    fn new_manager_should_work() {
        let manager = UnitManager::new();
        assert_eq!(manager.units.len(), 0);
    }

    #[test]
    fn add_unit_changes_should_work() {
        let mut manager = UnitManager::new();
        assert_eq!(manager.units.len(), 0);

        let (unit1, unit2) = build_unitrefs();

        manager.add_unit(unit1.clone());
        assert_eq!(manager.units.len(), 1);

        manager.add_unit(unit2.clone());
        assert_eq!(manager.units.len(), 2);
    }

    #[test]
    fn stop_request_should_work() {
        let mut manager = UnitManager::new();
        assert_eq!(*manager.stop_requested.lock().unwrap(), false);

        manager.request_stop();
        assert_eq!(*manager.stop_requested.lock().unwrap(), true);
    }

    #[test]
    fn reset_stop_request_should_work() {
        let mut manager = UnitManager::new();
        assert_eq!(*manager.stop_requested.lock().unwrap(), false);

        manager.request_stop();
        assert_eq!(*manager.stop_requested.lock().unwrap(), true);

        manager.reset_stop_request();
        assert_eq!(*manager.stop_requested.lock().unwrap(), false);
    }

    #[test]
    fn start_all_should_work() {
        let mut manager = UnitManager::new();
        let (unit1, unit2) = build_unitrefs();

        manager.add_unit(unit1.clone());
        manager.add_unit(unit2.clone());
        assert_eq!(manager.units.len(), 2);

        manager.start_units();
        assert_eq!(unit1.lock().unwrap().is_running(), true);
        assert_eq!(unit2.lock().unwrap().is_running(), true);
    }

    #[test]
    fn all_units_running_should_work() {
        let mut manager = UnitManager::new();
        let (unit1, unit2) = build_unitrefs();

        manager.add_unit(unit1.clone());
        manager.add_unit(unit2.clone());
        assert_eq!(manager.units.len(), 2);

        assert_eq!(manager.all_units_running(), false);
        manager.start_units();
        assert_eq!(manager.all_units_running(), true);
    }

    #[test]
    fn stop_all_should_work() {
        let mut manager = UnitManager::new();
        let (unit1, unit2) = build_unitrefs();

        manager.add_unit(unit1.clone());
        manager.add_unit(unit2.clone());
        assert_eq!(manager.units.len(), 2);

        manager.start_units();
        assert_eq!(unit1.lock().unwrap().is_running(), true);
        assert_eq!(unit2.lock().unwrap().is_running(), true);

        manager.stop_units();
        assert_eq!(unit1.lock().unwrap().is_running(), false);
        assert_eq!(unit2.lock().unwrap().is_running(), false);
    }

    #[test]
    fn all_units_stopped_should_work() {
        let mut manager = UnitManager::new();
        let (unit1, unit2) = build_unitrefs();

        manager.add_unit(unit1.clone());
        manager.add_unit(unit2.clone());
        assert_eq!(manager.units.len(), 2);

        manager.start_units();
        assert_eq!(manager.all_units_running(), true);

        manager.stop_units();
        assert_eq!(manager.all_units_stopped(), true);
    }
}

