use std::sync::{Arc, Mutex};
use log::{debug, error, warn};

use crate::unit::UnitRef;


pub type ManagerRef = Arc<Mutex<Manager>>;


pub struct Manager {
    units: Vec<UnitRef>,
    stop_requested: Arc<Mutex<bool>>,
}


impl Manager {
    pub fn new() -> Manager {
        Manager {
            units: Vec::new(),
            stop_requested: Arc::new(Mutex::new(false)),
        }
    }

    pub fn new_ref() -> ManagerRef {
        Arc::new(Mutex::new(Manager::new()))
    }

    pub fn add_unit(&mut self, unit: UnitRef) {
        debug!("Adding unit {:?}", unit.lock().unwrap());
        self.units.push(unit);
    }

    pub fn units(&self) -> &Vec<UnitRef> {
        &self.units
    }

    pub fn stop_requested(&self) -> &Arc<Mutex<bool>> {
        &self.stop_requested
    }

    /// Set should_stop flag to true
    /// This will stop the thread that is started by start_all_thread()
    pub fn request_stop(&mut self) {
        let mut should_stop = self.stop_requested.lock().unwrap();
        *should_stop = true;
    }

    /// reset should_stop to false
    pub fn reset_stop_request(&mut self) {
        let mut should_stop = self.stop_requested.lock().unwrap();
        *should_stop = false;
    }

    /// Iterate over all units and try to start them
    /// Note that we need to call this function several times until all dependencies are started
    pub fn start_all(&mut self) {
        let mut should_stop = self.stop_requested.lock().unwrap();
        *should_stop = false;

        for unit in &self.units {
            match unit.lock() {
                Ok(mut unit) => {
                    match unit.start() {
                        Ok(_) => {
                            debug!("Started unit {}", unit.name());
                        }
                        Err(e) => {
                            warn!("Error starting unit {}: {}", unit.name(), e);
                        }
                    }
                }
                Err(e) => {
                    error!("Error acquiring lock while starting unit: {}", e);
                }
            }
        }
    }

    /// Iterate over all units and try to stop them
    /// Note that we need to call this function several times until all dependencies are stopped
    pub fn stop_all(&mut self) {
        for unit in &self.units {
            match unit.lock() {
                Ok(mut unit) => {
                    match unit.stop() {
                        Ok(_) => {
                            debug!("Stopped unit {}", unit.name());
                        }
                        Err(e) => {
                            warn!("Error stopping unit {}: {}", unit.name(), e);
                        }
                    }
                }
                Err(e) => {
                    error!("Error acquiring lock while stopping unit: {}", e);
                }
            }
        }
    }

    /// Returns true if all units are running
    /// Returns false if at least one unit is not running
    pub fn all_units_running(&self) -> Result<bool, String> {
        for unit in &self.units {
            match unit.lock() {
                Ok(mut unit) => {
                    if !unit.test_running() {
                        return Ok(false);
                    }
                }
                Err(e) => {
                    return Err(format!("Error acquiring lock while checking if all units are running: {}", e));
                }
            }
        }

        return Ok(true);
    }

    /// Returns true if all units are stopped
    /// Returns false if at least one unit is not stopped
    pub fn all_units_stopped(&self) -> Result<bool, String> {
        for unit in &self.units {
            match unit.lock() {
                Ok(mut unit) => {
                    if unit.test_running() {
                        return Ok(false);
                    }
                }
                Err(e) => {
                    return Err(format!("Error acquiring lock while checking if all units are stopped: {}", e));
                }
            }
        }

        return Ok(true);
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
        );

        let unit2 = Unit::new_ref(
            String::from("test2"),
            String::from("sleep"),
            vec![String::from("1")],
            RestartPolicy::Never,
            get_current_uid(),
            get_current_gid(),
            true,
        );

        unit2.lock().unwrap().add_dependency(unit1.clone());

        return (unit1, unit2);
    }

    #[test]
    fn new_manager_should_work() {
        let manager = Manager::new();
        assert_eq!(manager.units.len(), 0);
    }

    #[test]
    fn add_unit_changes_should_work() {
        let mut manager = Manager::new();
        assert_eq!(manager.units.len(), 0);

        let (unit1, unit2) = build_unitrefs();

        manager.add_unit(unit1.clone());
        assert_eq!(manager.units.len(), 1);

        manager.add_unit(unit2.clone());
        assert_eq!(manager.units.len(), 2);
    }

    #[test]
    fn stop_request_should_work() {
        let mut manager = Manager::new();
        assert_eq!(*manager.stop_requested.lock().unwrap(), false);

        manager.request_stop();
        assert_eq!(*manager.stop_requested.lock().unwrap(), true);
    }

    #[test]
    fn reset_stop_request_should_work() {
        let mut manager = Manager::new();
        assert_eq!(*manager.stop_requested.lock().unwrap(), false);

        manager.request_stop();
        assert_eq!(*manager.stop_requested.lock().unwrap(), true);

        manager.reset_stop_request();
        assert_eq!(*manager.stop_requested.lock().unwrap(), false);
    }

    #[test]
    fn start_all_should_work() {
        let mut manager = Manager::new();
        let (unit1, unit2) = build_unitrefs();

        manager.add_unit(unit1.clone());
        manager.add_unit(unit2.clone());
        assert_eq!(manager.units.len(), 2);

        manager.start_all();
        assert_eq!(unit1.lock().unwrap().test_running(), true);
        assert_eq!(unit2.lock().unwrap().test_running(), true);
    }

    #[test]
    fn all_units_running_should_work() {
        let mut manager = Manager::new();
        let (unit1, unit2) = build_unitrefs();

        manager.add_unit(unit1.clone());
        manager.add_unit(unit2.clone());
        assert_eq!(manager.units.len(), 2);

        assert_eq!(manager.all_units_running().unwrap(), false);
        manager.start_all();
        assert_eq!(manager.all_units_running().unwrap(), true);
    }

    #[test]
    fn stop_all_should_work() {
        let mut manager = Manager::new();
        let (unit1, unit2) = build_unitrefs();

        manager.add_unit(unit1.clone());
        manager.add_unit(unit2.clone());
        assert_eq!(manager.units.len(), 2);

        manager.start_all();
        assert_eq!(unit1.lock().unwrap().test_running(), true);
        assert_eq!(unit2.lock().unwrap().test_running(), true);

        manager.stop_all();
        assert_eq!(unit1.lock().unwrap().test_running(), false);
        assert_eq!(unit2.lock().unwrap().test_running(), false);
    }

    #[test]
    fn all_units_stopped_should_work() {
        let mut manager = Manager::new();
        let (unit1, unit2) = build_unitrefs();

        manager.add_unit(unit1.clone());
        manager.add_unit(unit2.clone());
        assert_eq!(manager.units.len(), 2);

        manager.start_all();
        assert_eq!(manager.all_units_running().unwrap(), true);

        manager.stop_all();
        assert_eq!(manager.all_units_stopped().unwrap(), true);
    }
}

