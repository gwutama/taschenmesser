use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use sysinfo::{Pid, PidExt, ProcessRefreshKind, System, SystemExt};
use log::{debug, warn, trace, error};

use crate::unit::ProbeState;


pub type ProcessProbeRef = Arc<Mutex<ProcessProbe>>;


#[derive(Clone)]
pub struct ProcessProbe {
    pid: Option<i32>,
    interval_s: i32,
    system_info: Arc<Mutex<System>>,
    probe_state: ProbeState,
    stop_requested: Arc<Mutex<bool>>,
}


impl ProcessProbe {
    pub fn new(
        pid: Option<i32>,
        interval_s: i32,
        system_info: Arc<Mutex<System>>,
    ) -> ProcessProbe {
        return ProcessProbe {
            pid,
            interval_s,
            system_info,
            probe_state: ProbeState::Undefined,
            stop_requested: Arc::new(Mutex::new(false)),
        };
    }

    pub fn new_ref(
        pid: Option<i32>,
        interval_s: i32,
        system_info: Arc<Mutex<System>>,
    ) -> ProcessProbeRef {
        return Arc::new(Mutex::new(ProcessProbe::new(
            pid,
            interval_s,
            system_info,
        )));
    }

    fn stop_requested(&self) -> bool {
        return match self.stop_requested.lock() {
            Ok(should_stop) => *should_stop,
            Err(e) => {
                error!("Failed to lock stop_requested: {}", e);
                false
            }
        };
    }

    /// Set should_stop flag to true
    pub fn request_stop(&mut self) {
        match self.stop_requested.lock() {
            Ok(mut should_stop) => *should_stop = true,
            Err(e) => error!("Failed to lock stop_requested: {}", e),
        };
    }

    pub fn run(&self) -> JoinHandle<()> {
        let mut self_clone = self.clone();
        return thread::spawn(move || self_clone.run_loop());
    }

    /// Run probe() endlessly in a loop
    /// interval_s: 0 means no interval (run once)
    fn run_loop(&mut self) {
        loop {
            if self.stop_requested() {
                debug!("Stop requested");
                break;
            }

            self.probe(self.pid);

            if self.interval_s == 0 {
                break;
            }

            thread::sleep(Duration::from_secs(self.interval_s as u64));
        }
    }

    fn probe(&mut self, pid: Option<i32>) {
        match pid {
            Some(pid_val) => {
                if self.pid_exists(pid_val) {
                    debug!("Pid {} exists", pid_val);
                    self.probe_state = ProbeState::Alive;
                } else {
                    warn!("Pid {} does not exist", pid_val);
                    self.probe_state = ProbeState::Dead;
                }
            }
            None => {
                self.probe_state = ProbeState::Undefined
            },
        }
    }

    fn pid_exists(&mut self, pid: i32) -> bool {
        return match self.system_info.lock() {
            Ok(mut system_info) => {
                let sysinfo_pid = Pid::from_u32(pid as u32);
                let refresh = ProcessRefreshKind::new();
                let process_exists = system_info.refresh_process_specifics(sysinfo_pid, refresh);

                if process_exists {
                    trace!("Pid {} exists", pid);
                    true
                } else {
                    trace!("Pid {} does not exist", pid);
                    false
                }
            },
            Err(e) => {
                error!("Failed to lock system_info: {}", e);
                false
            }
        }
    }
}
