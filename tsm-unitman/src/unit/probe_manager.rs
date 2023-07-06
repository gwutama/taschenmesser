use log::{debug, warn, trace};

use crate::unit::{ProbeState, LivenessProbe, ProcessProbe};


pub struct ProbeManager {
    unit_name: String,
    liveness_probe: Option<LivenessProbe>,
    process_probe: Option<ProcessProbe>,
}


impl ProbeManager {
    pub fn new(unit_name: String) -> ProbeManager {
        return ProbeManager {
            unit_name,
            liveness_probe: None,
            process_probe: None,
        };
    }

    pub fn set_liveness_probe(&mut self, probe: LivenessProbe) {
        self.liveness_probe = Some(probe);
    }

    pub fn set_process_probe(&mut self, probe: ProcessProbe) {
        self.process_probe = Some(probe);
    }

    pub fn get_process_probe_state(&self) -> ProbeState {
        return match self.process_probe {
            Some(ref process_probe) => {
                process_probe.get_state()
            },
            None => {
                trace!("Unit {} does not have a process probe", self.unit_name);
                ProbeState::Undefined
            }
        };
    }

    pub fn get_liveness_probe_state(&self) -> ProbeState {
        return match self.liveness_probe {
            Some(ref liveness_probe) => {
                liveness_probe.get_state()
            },
            None => {
                trace!("Unit {} does not have a liveness probe", self.unit_name);
                ProbeState::Undefined
            }
        };
    }

    pub fn start_probes(&mut self) {
        // Start process probe
        match self.process_probe {
            Some(ref process_probe) => {
                process_probe.run();
            },
            None => {
                warn!("Cannot start process probe for unit {} because it is NOT defined", self.unit_name);
            }
        }

        // Start liveness probe
        match self.liveness_probe {
            Some(ref liveness_probe) => {
                liveness_probe.run();
            },
            None => {
                warn!("Cannot start liveness probe for unit {} because it is NOT defined", self.unit_name);
            }
        }
    }

    pub fn stop_probes(&mut self) {
        // Start process probe
        match self.process_probe {
            Some(ref mut process_probe) => {
                process_probe.request_stop();
            },
            None => {
                warn!("Cannot start process probe for unit {} because it is NOT defined", self.unit_name);
            }
        }

        // Start liveness probe
        match self.liveness_probe {
            Some(ref mut liveness_probe) => {
                liveness_probe.request_stop();
            },
            None => {
                warn!("Cannot start liveness probe for unit {} because it is NOT defined", self.unit_name);
            }
        }
    }
}