use std::str::FromStr;
use serde::Deserialize;
use serde::de::Error;


#[derive(Debug, PartialEq, Clone)]
pub enum RestartPolicy {
    Always,
    Never,
}


impl<'de> Deserialize<'de> for RestartPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        RestartPolicy::from_str(&s).map_err(Error::custom)
    }
}


impl FromStr for RestartPolicy {
    type Err = String;

    fn from_str(policy: &str) -> Result<Self, Self::Err> {
        match policy.to_lowercase().as_str() {
            "always" => {
                Ok(RestartPolicy::Always)
            }
            "never" => {
                Ok(RestartPolicy::Never)
            }
            _ => {
                Err(format!("Invalid restart policy: {}", policy))
            }
        }
    }
}
