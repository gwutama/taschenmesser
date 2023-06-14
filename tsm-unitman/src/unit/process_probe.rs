use std::io::Error;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use process_control::{ChildExt, Control, ExitStatus};
use std::time::SystemTime;
use std::sync::{Arc, Mutex};
use log::{debug, warn, trace};


pub type ProcessProbeRef = Arc<Mutex<ProcessProbe>>;


#[derive(Debug, Clone)]
pub struct ProcessProbe {
    executable: String,
    arguments: Vec<String>,
    timeout_s: i32,
    interval_s: i32,
    probe_timestamp: Option<Instant>,
}


impl ProcessProbe {
    /// timeout_s: 0 means no timeout
    /// interval_s: 0 means no interval (run once)
    pub fn new(
        executable: String,
        arguments: Vec<String>,
        timeout_s: i32,
        interval_s: i32
    ) -> ProcessProbe {
        return ProcessProbe {
            executable,
            arguments,
            timeout_s,
            interval_s,
            probe_timestamp: Some(Instant::now()),
        };
    }

    pub fn new_ref(
        executable: String,
        arguments: Vec<String>,
        timeout_s: i32,
        interval_s: i32,
    ) -> ProcessProbeRef {
        return Arc::new(Mutex::new(ProcessProbe::new(
            executable,
            arguments,
            timeout_s,
            interval_s,
        )));
    }

    /// interval_s: 0 means no interval (run once)
    fn is_time_to_probe(&mut self) -> bool {
        let now = Instant::now();

        return match self.interval_s {
            0 => {
                if self.probe_timestamp.is_none() {
                    self.probe_timestamp = Some(now);
                    true
                } else {
                    false
                }
            },
            _ => {
                let secs_since = self.secs_since_last_probe(self.probe_timestamp);
                let probe_now = secs_since >= self.interval_s as u64;

                if probe_now {
                    self.probe_timestamp = Some(now);
                    true
                } else {
                    false
                }
            }
        }
    }

    fn secs_since_last_probe(&self, before: Option<Instant>) -> u64 {
        let now = Instant::now();

        return match before {
            Some(before) => {
                now.duration_since(before).as_secs()
            },
            None => 0
        }
    }

    /// Probe once and returns whether it was successful.
    /// timeout_s: 0 means no timeout
    /// Ok: true if process executed successfully, false if it is still not time to probe
    /// Error: process failed to execute, or timed out, or exited with non-zero exit code
    pub fn probe(&mut self) -> Result<bool, String> {
        if !self.is_time_to_probe() {
            trace!("Not time to probe yet");
            return Ok(false);
        }

        let timeout_s = match self.timeout_s {
            0 => 3600, // ok, not really a timeout but it is absurdly long enough
            _ => self.timeout_s as u64
        };

        let process = Command::new(&self.executable)
            .args(&self.arguments)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        return match process {
            Ok(mut child) => {
                let output_result: Result<Option<ExitStatus>, Error> = child
                    .controlled()
                    .time_limit(Duration::from_secs(timeout_s))
                    .terminate_for_timeout()
                    .wait();

                match output_result {
                    Ok(output) => {
                        match output {
                            Some(exit_status) => {
                                if exit_status.success() {
                                    trace!("Probe successful");
                                    Ok(true)
                                } else {
                                    Err(format!("Process exited with non-zero exit code: {}", exit_status))
                                }
                            }
                            None => {
                                Err(format!("Process timed out after {} seconds", self.timeout_s))
                            }
                        }
                    },
                    Err(e) => {
                        Err(format!("Failed executing command {}: {}", self.executable, e))
                    }
                }
            }
            Err(e) => {
                Err(format!("Failed executing command {}: {}", self.executable, e))
            }
        }
    }
}