use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use log::{debug, error, warn, info};

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

    pub fn start_unit(&self, name: String) -> Result<bool, String> {
        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => {
                    if unit.get_name() == name {
                        if unit.is_running() {
                            debug!("Unit {} is already running", unit.get_name());
                            return Ok(true);
                        }

                        debug!("Starting unit {}", unit.get_name());

                        match unit.start() {
                            Ok(_) => {
                                unit.set_restart_policy(RestartPolicy::DisabledTemporarily);
                                unit.start_probes();

                                debug!("Started unit {}", unit.get_name());
                                return Ok(true);
                            },
                            Err(e) => {
                                warn!("Error starting unit {}: {}", unit.get_name(), e);
                                return Err(format!("Error starting unit {}: {}", unit.get_name(), e));
                            },
                        }
                    }
                }
                Err(e) => {
                    warn!("Error acquiring lock while starting unit: {}", e);
                    return Err(format!("Error acquiring lock while starting unit: {}", e));
                },
            }
        }

        Err(format!("Unit {} not found", name))
    }

    /// Iterate over all units and try to start them
    /// Note that we need to call this function several times until all dependencies are started
    fn start_units(&mut self) {
        self.reset_stop_request();

        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => {
                    if unit.is_running() {
                        debug!("Unit {} is already running", unit.get_name());
                        continue;
                    }

                    debug!("Starting unit {}", unit.get_name());

                    match unit.start() {
                        Ok(_) => debug!("Started unit {}", unit.get_name()),
                        Err(e) => warn!("Error starting unit {}: {}", unit.get_name(), e),
                    }
                }
                Err(e) => error!("Error acquiring lock while starting unit: {}", e),
            }
        }
    }

    pub fn stop_unit(&self, name: String, restart: bool) -> Result<bool, String> {
        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => {
                    if unit.get_name() == name {
                        if !unit.is_running() {
                            debug!("Unit {} is already stopped", unit.get_name());
                            return Ok(true);
                        }

                        info!("Stopping unit {}", unit.get_name());

                        // stopping unit will automatically stop its probes and cleanup its resources
                        return match unit.stop() {
                            Ok(_) => {
                                if !restart {
                                    unit.set_restart_policy(RestartPolicy::DisabledTemporarily);
                                }

                                info!("Stopped unit {}", unit.get_name());
                                Ok(true)
                            },
                            Err(e) => {
                                warn!("Error stopping unit {}: {}", unit.get_name(), e);
                                Err(format!("Error stopping unit {}: {}", unit.get_name(), e))
                            },
                        }
                    }
                }
                Err(e) => {
                    warn!("Error acquiring lock while stopping unit: {}", e);
                    return Err(format!("Error acquiring lock while stopping unit: {}", e));
                },
            }
        }

        Err(format!("Unit {} not found", name))
    }

    /// Iterate over all units and try to stop them
    /// Units will be stopped regardless of their dependencies
    fn stop_units(&mut self) {
        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => {
                    info!("Stopping unit {}", unit.get_name());

                    // stopping unit will automatically stop its probes and cleanup its resources
                    match unit.stop() {
                        Ok(_) => info!("Stopped unit {}", unit.get_name()),
                        Err(e) => warn!("Error stopping unit {}: {}", unit.get_name(), e),
                    }
                }
                Err(e) => error!("Error acquiring lock while stopping unit: {}", e),
            }
        }

        thread::sleep(Duration::from_secs(1));
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
        info!("Starting units");
        self.start_units();

        info!("Starting unit probes");
        self.start_units_probes();

        info!("Monitoring units");
        loop {
            if self.stop_requested() {
                info!("Stop requested");
                break;
            }

            self.monitor();
            thread::sleep(Duration::from_secs(1));
        }

        info!("Shutting down units and their probes");
        self.stop_units();
        self.reset_stop_request();
    }

    fn start_units_probes(&self) {
        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => unit.start_probes(),
                Err(e) => error!("Error acquiring lock while starting unit probes: {}", e),
            }
        }
    }

    fn monitor(&self) {
        for unit in &self.units {
            match unit.try_lock() {
                Ok(mut unit) => {
                    let is_running = unit.is_running();

                    if !is_running {
                        debug!("Force stopping unit {} to make sure resources are cleaned up", unit.get_name());
                        match unit.stop() {
                            Ok(_) => debug!("Stopped unit {}", unit.get_name()),
                            Err(e) => warn!("Error stopping unit {}: {}", unit.get_name(), e),
                        }
                    }

                    if !is_running && unit.get_restart_policy() == RestartPolicy::Always {
                        debug!("Unit {} is not running, restarting because restart policy was set to Always.", unit.get_name());
                        match unit.restart() {
                            Ok(_) => {
                                unit.start_probes();
                                debug!("Unit {} restarted", unit.get_name());
                            },
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

    fn build_unitrefs() -> (Arc<Mutex<Unit>>, Arc<Mutex<Unit>>) {
        let unit1 = Arc::new(Mutex::new(Unit::new(
            String::from("test1"),
            String::from("sleep"),
            vec![String::from("1")],
            RestartPolicy::Always,
            get_current_uid(),
            get_current_gid(),
            true,
        )));

        let unit2 = Arc::new(Mutex::new(Unit::new(
            String::from("test2"),
            String::from("sleep"),
            vec![String::from("1")],
            RestartPolicy::Never,
            get_current_uid(),
            get_current_gid(),
            true,
        )));

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
}

