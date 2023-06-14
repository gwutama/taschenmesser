use serde::Deserialize;


#[derive(Deserialize, Debug)]
pub struct StartupProbe {
    command: Option<Vec<String>>,
}


#[derive(Deserialize, Debug)]
pub struct ReadinessProbe {
    command: Option<Vec<String>>,
}


#[derive(Deserialize, Debug)]
pub struct LivenessProbe {
    command: Option<Vec<String>>,
    interval_s: i32,
}
