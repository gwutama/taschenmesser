use std::sync::MutexGuard;
use std::thread::{sleep, spawn};
use std::thread::JoinHandle;
use std::time::Duration;
use crate::unit::manager::{Manager, ManagerRef};


pub struct Runner {}


impl Runner {
    /// Spawn a thread that executes UnitManager.start_all() within an infinite loop.
    /// The thread can be stopped by setting the should_stop flag to true.
    pub fn run_threaded(manager: ManagerRef) -> JoinHandle<()> {
        let thread_handle = spawn(move || {
            loop {
                if Self::test_should_stop(manager.clone()) {
                    break;
                }

                Self::start_units(manager.clone());
                sleep(Duration::from_millis(200));
            }

            Self::cleanup_units(manager.clone());
        });

        return thread_handle;
    }

    fn test_should_stop(manager: ManagerRef) -> bool {
        let manager_lock = manager.lock();

        return match manager_lock {
            Ok(manager) => {
                let should_stop = manager.stop_requested.lock();
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

    fn start_units(manager: ManagerRef) {
        let manager_lock = manager.lock();

        match manager_lock {
            Ok(mut manager) => {
                match manager.all_units_running() {
                    Ok(true) => {
                        println!("All units are running");
                    }
                    Ok(false) => {
                        println!("Not all units are running");
                        manager.start_all();
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

    fn cleanup_units(manager: ManagerRef) {
        let mut manager_lock = manager.lock();
        match manager_lock {
            Ok(mut manager) => {
                match manager.all_units_stopped() {
                    Ok(true) => {
                        println!("All units are stopped");
                    }
                    Ok(false) => {
                        Self::wait_stop_all_units(&mut manager);
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

    fn wait_stop_all_units(manager: &mut MutexGuard<Manager>) {
        println!("Stopping units");

        loop {
            match manager.all_units_stopped() {
                Ok(true) => {
                    println!("All units are stopped");
                    break;
                },
                Ok(false) => {
                    manager.stop_all()
                },
                Err(e) => {
                    println!("Error checking if all units are stopped: {}", e);
                    break;
                }
            }
        }
    }
}