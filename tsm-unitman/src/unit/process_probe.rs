use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use sysinfo::{Pid, PidExt, ProcessRefreshKind, System, SystemExt};
use log::{debug, warn, error};

use crate::unit::ProbeState;


#[derive(Clone, Debug)]
pub struct ProcessProbe {
    name: String,
    pid: u32,
    interval_s: i32,
    system_info: Arc<Mutex<System>>,
    state: Arc<Mutex<ProbeState>>,
    stop_requested: bool,
    probe_timestamp: Instant,
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
            stop_requested: false,
            probe_timestamp: Instant::now(),
        };
    }

    pub fn get_state(&self) -> ProbeState {
        match self.state.try_lock() {
            Ok(state) => state.clone(),
            Err(e) => {
                error!("Liveness probe for unit {} failed to lock state: {}", self.name, e);
                ProbeState::Undefined
            }
        }
    }

    fn set_state(&mut self, new_state: ProbeState) {
        match self.state.try_lock() {
            Ok(mut state) => *state = new_state.clone(),
            Err(e) => {
                error!("Liveness probe for unit {} failed to lock state: {}", self.name, e)
            },
        };
    }

    /// Set stop_requested flag to true
    pub fn request_stop(&mut self) {
        self.stop_requested = true;
    }

    pub fn run(&self) -> JoinHandle<()> {
        let mut self_clone = self.clone();
        return thread::spawn(move || self_clone.run_loop());
    }

    /// Run probe() endlessly in a loop
    fn run_loop(&mut self) {
        debug!("Process probe for unit {} starting", self.name);

        loop {
            if self.stop_requested {
                debug!("Process probe for unit {} stop requested", self.name);
                break;
            }

            if self.is_time_to_probe() {
                self.probe(self.pid);
                self.probe_timestamp = Instant::now();
            }

            thread::sleep(Duration::from_millis(500));
        }

        self.set_state(ProbeState::Dead);
        self.stop_requested = false;

        debug!("Process probe for unit {} stopped", self.name);
    }

    fn is_time_to_probe(&self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.probe_timestamp);
        return elapsed.as_secs() >= self.interval_s as u64;
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
