#[derive(Debug, PartialEq)]
pub enum ProbeState {
    Unknown,
    Startup,
    Ready,
    Alive,
}