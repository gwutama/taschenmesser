use std::io::ErrorKind;
use std::process::{Child, Command, Stdio, ExitStatus};
use std::os::unix::process::CommandExt;
use std::time::{Duration, Instant};
use log::{warn, debug};


#[derive(Debug)]
pub struct Process {
    executable: String,
    arguments: Vec<String>,
    uid: u32,
    gid: u32,
    child: Option<Child>,
    start_timestamp: Option<Instant>,
}


impl Process {
    pub fn new(
        executable: String,
        arguments: Vec<String>,
        uid: u32,
        gid: u32,
    ) -> Process {
        return Process {
            executable,
            arguments,
            uid,
            gid,
            child: None,
            start_timestamp: None,
        };
    }

    pub fn get_executable(&self) -> String {
        self.executable.clone()
    }

    pub fn get_arguments(&self) -> Vec<String> {
        self.arguments.clone()
    }

    pub fn get_uid(&self) -> u32 {
        self.uid
    }

    pub fn get_gid(&self) -> u32 {
        self.gid
    }

    pub fn get_pid(&self) -> Option<u32> {
        return match self.child {
            Some(ref child) => Some(child.id()),
            None => None,
        };
    }

    pub fn get_uptime(&self) -> Option<Duration> {
        return match self.start_timestamp {
            Some(timestamp) => {
                let uptime = Instant::now().duration_since(timestamp);
                return Some(uptime);
            }
            None => None,
        };
    }

    /// Checks if the process is running.
    pub fn is_running(&mut self) -> bool {
        // Process is not running because its pid doesn't exist
        if self.get_pid().is_none() {
            return false;
        }

        // Process is not running anymore because we have exit code
        if self.exit_code().is_some() {
            return false;
        }

        return true;
    }

    pub fn exit_code(&mut self) -> Option<ExitStatus> {
        return match self.child {
            Some(ref mut child) => {
                match child.try_wait() {
                    Ok(Some(exit_code)) => {
                        // Process is not running anymore
                        self.cleanup();
                        debug!("Process {} exited with code {}", self.executable, exit_code);
                        Some(ExitStatus::from(exit_code))
                    }
                    Ok(None) | Err(_) => None,
                }
            }
            None => None,
        };
    }

    /// Starts the child process
    pub fn start(&mut self) -> Result<bool, String> {
        if self.is_running() {
            debug!("Cannot start process {} because it is already running", self.executable);
            return Ok(false);
        }

        let child = Command::new(&self.executable)
            .args(&self.arguments)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .uid(self.uid)
            .gid(self.gid)
            .spawn();

        match child {
            Ok(child) => {
                debug!("Process {} was started", self.executable);
                self.child = Some(child);
                self.start_timestamp = Some(Instant::now());
                Ok(true)
            }
            Err(error) => {
                self.cleanup();
                Err(format!("Process {} failed to start: {}", self.executable, error))
            }
        }
    }

    /// Stops the child process
    pub fn stop(&mut self) -> Result<bool, String> {
        if !self.is_running() {
            debug!("Cannot stop process {} because it is not running", self.executable);
            return Ok(false);
        }

        match self.child {
            Some(ref mut child) => {
                match child.kill() {
                    Ok(_) => {
                        match child.wait() {
                            Ok(_) => {
                                debug!("Process {} was stopped", self.executable);
                                self.cleanup();
                                Ok(true)
                            }
                            Err(error) => {
                                Err(format!("Process {} failed to wait: {}", self.executable, error))
                            }
                        }
                    }
                    Err(error) => {
                        if error.kind() == ErrorKind::InvalidInput {
                            debug!("Process {} has already stopped", self.executable);
                            self.cleanup();
                            return Ok(true);
                        }

                        Err(format!("Process {} failed to stop: {}", self.executable, error))
                    }
                }
            }
            None => {
                warn!("Cannot stop process {} because it is NOT running", self.executable);
                Ok(false)
            }
        }
    }

    pub fn restart(&mut self) -> Result<bool, String> {
        debug!("Restarting process {}", self.executable);
        match self.stop() {
            Ok(_) => {
                match self.start() {
                    Ok(_) => {
                        Ok(true)
                    }
                    Err(error) => {
                        Err(error)
                    }
                }
            }
            Err(error) => {
                Err(error)
            }
        }
    }

    fn cleanup(&mut self) {
        self.start_timestamp = None;
        self.cleanup_process_handles();
    }

    fn cleanup_process_handles(&mut self) {
        match self.child {
            Some(ref mut child) => {
                match child.stdout.take() {
                    Some(stdout) => {
                        drop(stdout);
                    }
                    None => {}
                }

                match child.stderr.take() {
                    Some(stderr) => {
                        drop(stderr);
                    }
                    None => {}
                }

                self.child = None;
            }
            None => {}
        }
    }
}