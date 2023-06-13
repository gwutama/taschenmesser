use std::sync::MutexGuard;
use std::thread::{sleep, spawn};
use std::thread::JoinHandle;
use std::time::Duration;
use crate::unit::manager::{Manager, ManagerRef};
use crate::unit::unit::{Unit, UnitRef, RestartPolicy};


pub struct Runner {}

const LOG_TAG: &str = "[unit::Runner]";


impl Runner {
    /// Spawn a thread that executes UnitManager.start_all() within an infinite loop.
    /// The thread can be stopped by setting the should_stop flag to true.
    pub fn run_threaded(manager: ManagerRef) -> JoinHandle<()> {
        println!("{} Spawning thread", LOG_TAG);

        let thread_handle = spawn(move || {
            println!("{} Starting units", LOG_TAG);
            Self::startup_units(manager.clone());

            println!("{} Supervising units", LOG_TAG);
            loop {
                if Self::test_stop_request(manager.clone()) {
                    println!("{} Stop requested", LOG_TAG);
                    break;
                }

                Self::watch_units(manager.clone());
                sleep(Duration::from_millis(200));
            }

            println!("{} Shutting down units", LOG_TAG);
            Self::shutdown_units(manager.clone());
            Self::reset_stop_request(manager.clone());
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
                        println!("{} Error acquiring lock while testing stop_request: {}", LOG_TAG, e);
                        false
                    }
                }
            }
            Err(e) => {
                println!("{} Error acquiring lock while testing stop_request: {}", LOG_TAG, e);
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
                println!("{} Error acquiring lock while resetting stop_request: {}", LOG_TAG, e);
            }
        }
    }

    fn startup_units(manager: ManagerRef) {
        let manager_lock = manager.lock();
        match manager_lock {
            Ok(mut manager) => {
                match manager.all_units_running() {
                    Ok(true) => {
                        println!("{} All units are started", LOG_TAG);
                    }
                    Ok(false) => {
                        Self::wait_start_all_units(&mut manager);
                    }
                    Err(e) => {
                        println!("{} Error checking if all units are started: {}", LOG_TAG, e);
                    }
                }
            }
            Err(e) => {
                println!("{} Error acquiring lock while starting up units: {}", LOG_TAG, e);
            }
        }
    }

    fn wait_start_all_units(manager: &mut MutexGuard<Manager>) {
        loop {
            match manager.all_units_running() {
                Ok(true) => {
                    println!("{} All units are started", LOG_TAG);
                    break;
                },
                Ok(false) => {
                    manager.start_all()
                },
                Err(e) => {
                    println!("{} Error checking if all units are started: {}", LOG_TAG, e);
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
                let units = manager.units().clone();

                for unit in units {
                    match unit.lock() {
                        Ok(mut unit) => {
                            if *unit.restart_policy() == RestartPolicy::Always && !unit.test_running() {
                                println!("{} Unit {} is not running, restarting", LOG_TAG, unit.name());
                                let start_result = unit.start();

                                match start_result {
                                    Ok(_) => {
                                        println!("{} Unit {} restarted", LOG_TAG, unit.name());
                                    }
                                    Err(e) => {
                                        println!("{} Error restarting unit {}: {}", LOG_TAG, unit.name(), e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("{} Error acquiring lock while watching units: {}", LOG_TAG, e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("{} Error acquiring lock while watching units: {}", LOG_TAG, e);
            }
        }
    }

    fn shutdown_units(manager: ManagerRef) {
        let manager_lock = manager.lock();
        match manager_lock {
            Ok(mut manager) => {
                match manager.all_units_stopped() {
                    Ok(true) => {
                        println!("{} All units are stopped", LOG_TAG);
                    }
                    Ok(false) => {
                        Self::wait_stop_all_units(&mut manager);
                    }
                    Err(e) => {
                        println!("{} Error checking if all units are stopped: {}", LOG_TAG, e);
                    }
                }
            }
            Err(e) => {
                println!("{} Error acquiring lock while cleaning up units: {}", LOG_TAG, e);
            }
        }
    }

    fn wait_stop_all_units(manager: &mut MutexGuard<Manager>) {
        loop {
            match manager.all_units_stopped() {
                Ok(true) => {
                    println!("{} All units are stopped", LOG_TAG);
                    break;
                },
                Ok(false) => {
                    manager.stop_all()
                },
                Err(e) => {
                    println!("{} Error checking if all units are stopped: {}", LOG_TAG, e);
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use users::{get_current_uid, get_current_gid};

    fn build_unitrefs() -> Vec<UnitRef> {
        let unit1 = Unit::new_ref(
            String::from("test1"),
            String::from("sleep"),
            vec![String::from("5")],
            RestartPolicy::Always,
            get_current_uid(),
            get_current_gid(),
            true,
        );

        let unit2 = Unit::new_ref(
            String::from("test2"),
            String::from("sleep"),
            vec![String::from("5")],
            RestartPolicy::Never,
            get_current_uid(),
            get_current_gid(),
            true,
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