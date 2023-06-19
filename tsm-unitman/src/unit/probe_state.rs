#[derive(Debug, PartialEq, Clone)]
pub enum ProbeState {
    Undefined, // the probe is not configured
    Alive, // the probe is configured and the process is running
    Dead, // the probe is configured and the process is not running
}