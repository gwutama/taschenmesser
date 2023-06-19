use std::sync::MutexGuard;
use std::thread::{sleep, spawn};
use std::thread::JoinHandle;
use std::time::Duration;
use log::{debug, error, warn};

use crate::unit::{Manager, ManagerRef, RestartPolicy};


pub struct Runner {}


impl Runner {
    pub fn run(manager: ManagerRef) {
        debug!("Starting units");
        Self::startup_units(manager.clone());

        debug!("Supervising units");
        loop {
            if Self::test_stop_request(manager.clone()) {
                debug!("Stop requested");
                break;
            }

            Self::watch_units(manager.clone());
            sleep(Duration::from_secs(1));
        }

        debug!("Shutting down units");
        Self::shutdown_units(manager.clone());
        Self::reset_stop_request(manager.clone());
    }

    /// Spawn a thread that executes UnitManager.start_all() within an infinite loop.
    /// The thread can be stopped by setting the should_stop flag to true.
    pub fn run_threaded(manager: ManagerRef) -> JoinHandle<()> {
        debug!("Spawning thread");

        let thread_handle = spawn(move || {
            Self::run(manager);
        });

        return thread_handle;
    }

    fn test_stop_request(manager: ManagerRef) -> bool {
        let manager_lock = manager.lock();

        return match manager_lock {
            Ok(manager) => {
                let should_stop = manager.stop_requested().lock();
                match should_stop {
                    Ok(should_stop) => {
                        *should_stop
                    }
                    Err(e) => {
                        error!("Error acquiring lock while testing stop_request: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                error!("Error acquiring lock while testing stop_request: {}", e);
                false
            }
        }
    }

    fn reset_stop_request(manager: ManagerRef) {
        let manager_lock = manager.lock();

        match manager_lock {
            Ok(mut manager) => {
                manager.reset_stop_request();
            }
            Err(e) => {
                error!("Error acquiring lock while resetting stop_request: {}", e);
            }
        }
    }

    fn startup_units(manager: ManagerRef) {
        let manager_lock = manager.lock();
        match manager_lock {
            Ok(mut manager) => {
                match manager.all_units_running() {
                    Ok(true) => {
                        debug!("All units are started");
                    }
                    Ok(false) => {
                        Self::wait_start_all_units(&mut manager);
                    }
                    Err(e) => {
                        warn!("Error checking if all units are started: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Error acquiring lock while starting up units: {}", e);
            }
        }
    }

    fn wait_start_all_units(manager: &mut MutexGuard<Manager>) {
        loop {
            match manager.all_units_running() {
                Ok(true) => {
                    debug!("All units are started");
                    break;
                },
                Ok(false) => {
                    manager.start_all()
                },
                Err(e) => {
                    warn!("Error checking if all units are started: {}", e);
                    break;
                }
            }
        }
    }

    /// Watch every unit and restart it if it is not running anymore
    fn watch_units(manager: ManagerRef) {
        let manager_lock = manager.lock();

        match manager_lock {
            Ok(manager) => {
                let units = manager.get_units().clone();

                for unit in units {
                    match unit.lock() {
                        Ok(mut unit) => {
                            unit.liveness_probe();

                            if !unit.test_running() && unit.get_restart_policy() == RestartPolicy::Always {
                                debug!("Unit {} is not running, restarting because restart policy was set to Always.", unit.get_name());
                                let start_result = unit.start();

                                match start_result {
                                    Ok(_) => {
                                        debug!("Unit {} restarted", unit.get_name());
                                    }
                                    Err(e) => {
                                        warn!("Error restarting unit {}: {}", unit.get_name(), e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error acquiring lock while watching units: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error acquiring lock while watching units: {}", e);
            }
        }
    }

    fn shutdown_units(manager: ManagerRef) {
        let manager_lock = manager.lock();
        match manager_lock {
            Ok(mut manager) => {
                match manager.all_units_stopped() {
                    Ok(true) => {
                        debug!("All units are stopped");
                    }
                    Ok(false) => {
                        Self::wait_stop_all_units(&mut manager);
                    }
                    Err(e) => {
                        warn!("Error checking if all units are stopped: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Error acquiring lock while cleaning up units: {}", e);
            }
        }
    }

    fn wait_stop_all_units(manager: &mut MutexGuard<Manager>) {
        loop {
            match manager.all_units_stopped() {
                Ok(true) => {
                    debug!("All units are stopped");
                    break;
                },
                Ok(false) => {
                    manager.stop_all()
                },
                Err(e) => {
                    error!("Error checking if all units are stopped: {}", e);
                    break;
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
    use crate::unit::unit::{Unit, UnitRef};

    fn build_unitrefs() -> Vec<UnitRef> {
        let unit1 = Unit::new_ref(
            String::from("test1"),
            String::from("sleep"),
            vec![String::from("5")],
            RestartPolicy::Always,
            get_current_uid(),
            get_current_gid(),
            true,
            None,
        );

        let unit2 = Unit::new_ref(
            String::from("test2"),
            String::from("sleep"),
            vec![String::from("5")],
            RestartPolicy::Never,
            get_current_uid(),
            get_current_gid(),
            true,
            None,
        );

        unit2.lock().unwrap().add_dependency(unit1.clone());

        let unit3 = Unit::new_ref(
            String::from("test3"),
            String::from("sleep"),
            vec![String::from("5")],
            RestartPolicy::Never,
            get_current_uid(),
            get_current_gid(),
            true,
            None,
        );

        unit3.lock().unwrap().add_dependency(unit1.clone());

        return vec![unit1, unit2, unit3];
    }

    #[test]
    fn run_threaded_should_work() {
        let manager = Manager::new_ref();

        let mut units = build_unitrefs();
        units.reverse();

        for unit in units {
            manager.lock().unwrap().add_unit(unit);
        }

        let thread_handle = Runner::run_threaded(manager.clone());
        sleep(Duration::from_millis(1000));
        manager.lock().unwrap().request_stop();

        thread_handle.join().unwrap();
    }
}