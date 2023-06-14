use std::io::Error;
use std::process::{Child, Command};
use std::time::Duration;
use serde::Deserialize;
use process_control::{ChildExt, Control, ExitStatus};
use log::{warn, debug};


#[derive(Deserialize, Debug, Clone)]
pub struct ProcessProbe {
    executable: String,
    arguments: Vec<String>,
    timeout_s: i32,
    interval_s: i32,
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
        };
    }

    /// executes probe_with_internal_and_timeout() within a thread
    pub fn probe(&self, success_callback_fn: fn()) {
        let cloned_self = self.clone();
        std::thread::spawn(move || {
            cloned_self.probe_with_interval_and_timeout(success_callback_fn);
        });
    }

    /// interval_s: 0 means no interval (run once)
    fn probe_with_interval_and_timeout(&self, success_callback_fn: fn()) {
        /// interval_s: 0 means no interval (run once)
        if self.interval_s == 0 {
            self.probe_with_timeout(success_callback_fn);
        }

        loop {
            self.probe_with_timeout(success_callback_fn);
            std::thread::sleep(Duration::from_secs(self.interval_s as u64));
        }
    }

    /// Probe once and returns whether it was successful.
    /// timeout_s: 0 means no timeout
    fn probe_with_timeout(&self, success_callback_fn: fn()) {
        let timeout_s: u64 = match self.timeout_s {
            0 => 3600, // ok, not really a timeout but it is absurdly long enough
            _ => self.timeout_s as u64
        };

        let process = Command::new(&self.executable)
            .args(&self.arguments)
            .spawn();

        return match process {
            Ok(mut child) => {
                let output_result: Result<Option<process_control::ExitStatus>, Error> = child
                    .controlled()
                    .time_limit(Duration::from_secs(timeout_s))
                    .terminate_for_timeout()
                    .wait();

                match output_result {
                    Ok(output) => {
                        match output {
                            Some(exit_status) => {
                                if exit_status.success() {
                                    success_callback_fn();
                                } else {
                                    warn!("Process exited with non-zero exit code: {}", exit_status);
                                }
                            }
                            None => {
                                warn!("Process timed out after {} seconds", self.timeout_s);
                            }
                        }
                    },
                    Err(e) => warn!("Failed executing command {}: {}", self.executable, e)
                }
            }
            Err(e) => {
                warn!("Failed executing command {}: {}", self.executable, e);
            }
        }
    }
}