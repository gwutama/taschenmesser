use std::sync::{Arc, LockResult, Mutex, MutexGuard};
use crate::unit::unit::UnitRef;


pub type ManagerRef = Arc<Mutex<Manager>>;


pub struct Manager {
    units: Vec<UnitRef>,
    pub should_stop: Arc<Mutex<bool>>,
}


impl Manager {
    pub fn new() -> Manager {
        Manager {
            units: Vec::new(),
            should_stop: Arc::new(Mutex::new(false)),
        }
    }

    pub fn add_unit(&mut self, unit: UnitRef) {
        self.units.push(unit);
    }

    /// Set should_stop flag to true
    /// This will stop the thread that is started by start_all_thread()
    pub fn stop(&mut self) {
        let mut should_stop = self.should_stop.lock().unwrap();
        *should_stop = true;
    }

    /// Iterate over all units and try to start them
    /// Note that we need to call this function several times until all dependencies are started
    pub fn start_all(&mut self) {
        let mut should_stop = self.should_stop.lock().unwrap();
        *should_stop = false;

        for unit in &self.units {
            match unit.lock() {
                Ok(mut unit) => {
                    match unit.start() {
                        Ok(_) => {
                            println!("Started unit {}", unit.name());
                        }
                        Err(e) => {
                            println!("Error starting unit {}: {}", unit.name(), e);
                        }
                    }
                }
                Err(e) => {
                    println!("Error acquiring lock while starting unit: {}", e);
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
                            println!("Stopped unit {}", unit.name());
                        }
                        Err(e) => {
                            println!("Error stopping unit {}: {}", unit.name(), e);
                        }
                    }
                }
                Err(e) => {
                    println!("Error acquiring lock while stopping unit: {}", e);
                }
            }
        }
    }

    /// Returns true if all units are running
    /// Returns false if at least one unit is not running
    pub fn all_units_running(&self) -> Result<bool, String> {
        for unit in &self.units {
            match unit.lock() {
                Ok(unit) => {
                    if !unit.is_running() {
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
                Ok(unit) => {
                    if unit.is_running() {
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

