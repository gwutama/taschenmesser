use std::process::{Child, Command, Stdio, ExitStatus};
use std::os::unix::process::CommandExt;
use std::sync::{Arc, Mutex};
use log::{debug, warn, trace};


pub type ProcessRef = Arc<Mutex<Process>>;


#[derive(Debug)]
pub struct Process {
    executable: String,
    arguments: Vec<String>,
    uid: u32,
    gid: u32,
    child: Option<Box<Child>>,
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
        };
    }

    pub fn new_ref(
        executable: String,
        arguments: Vec<String>,
        uid: u32,
        gid: u32,
    ) -> ProcessRef {
        return Arc::new(Mutex::new(Process::new(
            executable,
            arguments,
            uid,
            gid,
        )));
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
            Some(ref child) => Some(child.as_ref().id()),
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
            warn!("Cannot start process {} because it is already running", self.executable);
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
                self.child = Some(Box::new(child));
                Ok(true)
            }
            Err(error) => {
                self.child = None;
                Err(format!("Process {} failed to start: {}", self.executable, error))
            }
        }
    }

    /// Stops the child process
    pub fn stop(&mut self) -> Result<bool, String> {
        if !self.is_running() {
            warn!("Cannot stop process {} because it is NOT running", self.executable);
            return Ok(false);
        }

        match self.child {
            Some(ref mut child) => {
                match child.kill() {
                    Ok(_) => {
                        debug!("Process {} was stopped", self.executable);
                        self.cleanup_process_handles();
                        Ok(true)
                    }
                    Err(error) => {
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