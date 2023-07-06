use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use sysinfo::{Pid, PidExt, ProcessRefreshKind, System, SystemExt};
use log::{debug, warn, error};

use crate::unit::ProbeState;


pub type ProcessProbeRef = Arc<Mutex<ProcessProbe>>;


#[derive(Clone, Debug)]
pub struct ProcessProbe {
    name: String,
    pid: u32,
    interval_s: i32,
    system_info: Arc<Mutex<System>>,
    state: Arc<Mutex<ProbeState>>,
    stop_requested: Arc<Mutex<bool>>,
}


impl ProcessProbe {
    pub fn new(
        name: String,
        pid: u32,
        interval_s: i32,
    ) -> ProcessProbe {
        return ProcessProbe {
            name,
            pid,
            interval_s,
            system_info: Arc::new(Mutex::new(System::new())),
            state: Arc::new(Mutex::new(ProbeState::Undefined)),
            stop_requested: Arc::new(Mutex::new(false)),
        };
    }

    pub fn get_state(&self) -> ProbeState {
        match self.state.try_lock() {
            Ok(state) => state.clone(),
            Err(e) => {
                error!("Process probe for unit {} failed to lock state: {}", self.name, e);
                ProbeState::Undefined
            }
        }
    }

    fn set_state(&mut self, new_state: ProbeState) {
        match self.state.try_lock() {
            Ok(mut state) => *state = new_state.clone(),
            Err(e) => {
                error!("Process probe for unit {} failed to lock state: {}", self.name, e)
            },
        };
    }

    fn stop_requested(&self) -> bool {
        return match self.stop_requested.try_lock() {
            Ok(stop_requested) => *stop_requested,
            Err(e) => {
                error!("Process probe for unit {} failed to lock stop_requested: {}", self.name, e);
                false
            }
        };
    }

    /// Set stop_requested flag to true
    pub fn request_stop(&mut self) {
        match self.stop_requested.try_lock() {
            Ok(mut stop_requested) => *stop_requested = true,
            Err(e) => {
                error!("Process probe for unit {} failed to lock stop_requested: {}", self.name, e)
            },
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
                debug!("Process probe for unit {} stop requested", self.name);
                break;
            }

            self.probe(self.pid);

            if self.interval_s == 0 {
                break;
            }

            thread::sleep(Duration::from_secs(self.interval_s as u64));
        }
    }

    fn probe(&mut self, pid: u32) {
        if self.pid_exists(pid) {
            debug!("Process probe for unit {} succeeded (pid={}). Setting probe state to Alive.", self.name, pid);
            self.set_state(ProbeState::Alive);
        } else {
            warn!("Process probe for unit {} failed pid={} does not exist. Setting probe state to Dead.", self.name, pid);
            self.set_state(ProbeState::Dead);
        }
    }

    fn pid_exists(&self, pid: u32) -> bool {
        return match self.system_info.try_lock() {
            Ok(mut system_info) => {
                let sysinfo_pid = Pid::from_u32(pid);
                let refresh = ProcessRefreshKind::new();
                system_info.refresh_process_specifics(sysinfo_pid, refresh)
            },
            Err(e) => {
                error!("Process probe for unit {} failed to lock system_info: {}", self.name, e);
                false
            }
        }
    }
}
