use std::sync::MutexGuard;
use std::thread::{sleep, spawn};
use std::thread::JoinHandle;
use std::time::Duration;
use crate::unit_manager::{UnitManager, UnitManagerRef};


pub struct UnitManagerRunner {}


impl UnitManagerRunner {
    /// Spawn a thread that executes UnitManager.start_all() within an infinite loop.
    /// The thread can be stopped by setting the should_stop flag to true.
    pub fn run_threaded(unit_manager: UnitManagerRef) -> JoinHandle<()> {
        let thread_handle = spawn(move || {
            loop {
                if Self::test_should_stop(unit_manager.clone()) {
                    break;
                }

                Self::start_units(unit_manager.clone());
                sleep(Duration::from_millis(200));
            }

            Self::cleanup_units(unit_manager.clone());
        });

        return thread_handle;
    }

    fn test_should_stop(unit_manager: UnitManagerRef) -> bool {
        let unit_manager_lock = unit_manager.lock();

        return match unit_manager_lock {
            Ok(unit_manager) => {
                let should_stop = unit_manager.should_stop.lock();
                match should_stop {
                    Ok(should_stop) => {
                        *should_stop
                    }
                    Err(e) => {
                        println!("Error acquiring lock while testing should_stop: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                println!("Error acquiring lock while testing should_stop: {}", e);
                false
            }
        }
    }

    fn start_units(unit_manager: UnitManagerRef) {
        let unit_manager_lock = unit_manager.lock();

        match unit_manager_lock {
            Ok(mut unit_manager) => {
                match unit_manager.all_units_running() {
                    Ok(true) => {
                        println!("All units are running");
                    }
                    Ok(false) => {
                        println!("Not all units are running");
                        unit_manager.start_all();
                    }
                    Err(e) => {
                        println!("Error checking if all units are running: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Error acquiring lock while starting units: {}", e);
            }
        }
    }

    fn cleanup_units(unit_manager: UnitManagerRef) {
        let mut unit_manager_lock = unit_manager.lock();
        match unit_manager_lock {
            Ok(mut unit_manager) => {
                match unit_manager.all_units_stopped() {
                    Ok(true) => {
                        println!("All units are stopped");
                    }
                    Ok(false) => {
                        Self::wait_stop_all_units(&mut unit_manager);
                    }
                    Err(e) => {
                        println!("Error checking if all units are stopped: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Error acquiring lock while cleaning up units: {}", e);
            }
        }
    }

    fn wait_stop_all_units(unit_manager: &mut MutexGuard<UnitManager>) {
        println!("Stopping units");

        loop {
            match unit_manager.all_units_stopped() {
                Ok(true) => {
                    println!("All units are stopped");
                    break;
                },
                Ok(false) => {
                    unit_manager.stop_all()
                },
                Err(e) => {
                    println!("Error checking if all units are stopped: {}", e);
                    break;
                }
            }
        }
    }
}