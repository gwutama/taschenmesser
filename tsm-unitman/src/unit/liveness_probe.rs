use std::io::Error;
use std::process::{Command, Stdio};
use std::time::Duration;
use process_control::{ChildExt, Control, ExitStatus};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use log::{debug, warn, trace, error};
use crate::unit::ProbeState;


pub type LivenessProbeRef = Arc<Mutex<LivenessProbe>>;


#[derive(Debug, Clone)]
pub struct LivenessProbe {
    name: String,
    executable: String,
    arguments: Vec<String>,
    timeout_s: i32,
    interval_s: i32,
    state: Arc<Mutex<ProbeState>>,
    stop_requested: Arc<Mutex<bool>>,
}


impl LivenessProbe {
    /// timeout_s: 0 means no timeout
    /// interval_s: 0 means no interval (run once)
    pub fn new(
        name: String,
        executable: String,
        arguments: Vec<String>,
        timeout_s: i32,
        interval_s: i32
    ) -> LivenessProbe {
        return LivenessProbe {
            name,
            executable,
            arguments,
            timeout_s,
            interval_s,
            state: Arc::new(Mutex::new(ProbeState::Undefined)),
            stop_requested: Arc::new(Mutex::new(false)),
        };
    }

    pub fn new_ref(
        name: String,
        executable: String,
        arguments: Vec<String>,
        timeout_s: i32,
        interval_s: i32,
    ) -> LivenessProbeRef {
        return Arc::new(Mutex::new(LivenessProbe::new(
            name,
            executable,
            arguments,
            timeout_s,
            interval_s,
        )));
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
            Ok(mut stop_requested) => *stop_requested = true,
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

            self.probe();

            if self.interval_s == 0 {
                break;
            }

            thread::sleep(Duration::from_secs(self.interval_s as u64));
        }
    }

    /// timeout_s: 0 means no timeout
    /// Ok: true if process executed successfully, false if it is still not time to probe
    /// Error: process failed to execute, or timed out, or exited with non-zero exit code
    pub fn probe(&mut self) {
        let process = Command::new(&self.executable)
            .args(&self.arguments)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match process {
            Ok(mut child) => {
                let output_result: Result<Option<ExitStatus>, Error> = child
                    .controlled()
                    .time_limit(Duration::from_secs(self.timeout_s as u64))
                    .terminate_for_timeout()
                    .wait();

                match output_result {
                    Ok(output) => {
                        match output {
                            Some(exit_status) => {
                                if exit_status.success() {
                                    trace!("Liveness probe for unit {} succeeded. Setting probe state to Alive.", self.name);
                                    self.set_state(ProbeState::Alive);
                                } else {
                                    warn!("Liveness probe for unit {} failed with exit status {}. Setting probe state to Dead.", self.name, exit_status);
                                    self.set_state(ProbeState::Dead);
                                }
                            }
                            None => {
                                warn!("Liveness probe for unit {} failed due timeout. Setting probe state to Dead.", self.name);
                                self.set_state(ProbeState::Dead);
                            }
                        }
                    },
                    Err(e) => {
                        warn!("Liveness probe for unit {} failed: {}. Setting probe state to Undefined.", self.name, e);
                        self.set_state(ProbeState::Undefined);
                    }
                }
            }
            Err(e) => {
                warn!("Liveness probe for unit {} failed when executing command {}: {}. Setting probe state to Undefined.", self.name, self.executable, e);
                self.set_state(ProbeState::Undefined);
            }
        }
    }
}