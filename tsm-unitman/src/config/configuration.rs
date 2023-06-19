use std::collections::HashMap;
use std::fs;
use serde::Deserialize;
use log::{error, warn};

use crate::config::{Application, Unit, RpcServer};
use crate::unit;


#[derive(Deserialize, Debug)]
pub struct Configuration {
    pub application: Application,
    pub rpc_server: RpcServer,
    units: Vec<Unit>,
}


impl Configuration {
    pub fn from_file(file_path: String) -> Result<Configuration, String> {
        return match fs::read_to_string(file_path) {
            Ok(content) => {
                Configuration::from_string(content)
            },
            Err(error) => {
                Err(format!("Error reading configuration file: {}", error))
            }
        }
    }

    pub fn from_string(content: String) -> Result<Configuration, String> {
        return match toml::from_str(&content) {
            Ok(mut configuration) => {
                Ok(configuration)
            },
            Err(error) => {
                Err(format!("Error parsing configuration file: {}", error))
            }
        }
    }

    pub fn build_units(&self) -> Vec<unit::UnitRef> {
        let mut units = Vec::new();

        // In order to build the dependencies, we need to build all units first and push them
        // into a hash map. Then, we can iterate over the hash map and build the dependencies.
        let mut unit_map: HashMap<String, unit::UnitRef> = HashMap::new();

        for unit_configuration in &self.units {
            let unit_ref = unit_configuration.build_ref();

            units.push(unit_ref.clone());

            match unit_ref.lock() {
                Ok(unit) => {
                    unit_map.insert(unit.name().clone(), unit_ref.clone());
                },
                Err(e) => {
                    error!("Error acquiring lock while building unit ref: {}", e);
                }
            };
        }

        for unit_configuration in &self.units {
            let unit_ref = match unit_map.get(&unit_configuration.get_name()) {
                Some(unit_ref) => unit_ref,
                None => {
                    warn!("Unit {} not found in unit map", unit_configuration.get_name());
                    continue;
                }
            };

            // build dependencies
            for dependency_name in unit_configuration.get_dependencies() {
                let dependency_unit_ref = match unit_map.get(&dependency_name) {
                    Some(unit_ref) => unit_ref,
                    None => {
                        warn!("Dependency {} not found in unit map", dependency_name);
                        continue;
                    }
                };

                match unit_ref.lock() {
                    Ok(mut unit) => {
                        unit.add_dependency(dependency_unit_ref.clone());
                    },
                    Err(e) => {
                        error!("Error acquiring lock while building dependency for unit: {}", e);
                    }
                };
            }
        }

        return units;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LogLevel;

    fn sample_working_complete_conf() -> String {
        return String::from(
            r#"
                [application]
                log_level = "debug"

                [rpc_server]
                enabled = true
                bind_address = "ipc:///tmp/tsm-unitman.sock"

                [[units]]
                name = "foo"
                executable = "ls"
                arguments = [ "-l", "-a" ]
                dependencies = []
                restart_policy = "always"
                user = ""
                group = ""
                enabled = true
                liveness_probe.executable = "ls"
                liveness_probe.arguments = ["/tmp"]
                liveness_probe.interval_s = 5
                liveness_probe.timeout_s = 5

                [[units]]
                name = "bar"
                executable = "ps"
                arguments = [ "aux" ]
                dependencies = [ "foo" ]
                restart_policy = "never"
                user = ""
                group = ""
                enabled = true
                liveness_probe.executable = "ls"
                liveness_probe.arguments = ["/tmp"]
                liveness_probe.interval_s = 5
                liveness_probe.timeout_s = 5
            "#,
        );
    }

    fn sample_working_mandatory_only_conf() -> String {
        return String::from(
            r#"
                [application]

                [rpc_server]

                [[units]]
                name = "foo"
                executable = "ls"

                [[units]]
                name = "bar"
                executable = "ps"
                arguments = [ "aux" ]
                dependencies = [ "foo" ]
                restart_policy = "never"
                user = ""
                group = ""
                enabled = true
                liveness_probe.executable = "ls"
                liveness_probe.arguments = ["/tmp"]
                liveness_probe.interval_s = 5
                liveness_probe.timeout_s = 5
            "#,
        );
    }


    #[test]
    fn from_string_should_work() {
        let content= sample_working_complete_conf();
        let configuration = Configuration::from_string(content).unwrap();

        assert_eq!(configuration.application.get_log_level(), LogLevel::Debug);
        assert_eq!(configuration.units.len(), 2);
    }

    #[test]
    fn from_string_when_missing_optional_keys_should_work() {
        let content= sample_working_mandatory_only_conf();
        let configuration = Configuration::from_string(content).unwrap();

        assert_eq!(configuration.application.get_log_level(), LogLevel::Info);
    }

    #[test]
    fn from_file_should_work() {
        let file = String::from("resources/tsm-unitman.toml");
        let configuration = Configuration::from_file(file).unwrap();

        assert_eq!(configuration.application.get_log_level(), LogLevel::Debug);
        assert_eq!(configuration.units.len(), 2);
    }

    #[test]
    fn from_file_when_file_invalid_should_return_error() {
        let file = String::from("foo/bar/invalid.file");
        let configuration = Configuration::from_file(file);

        assert!(configuration.is_err());
    }

    #[test]
    fn build_units_should_work() {
        let content= sample_working_complete_conf();
        let configuration = Configuration::from_string(content).unwrap();

        let units = configuration.build_units();

        assert_eq!(units.len(), 2);
        assert_eq!(units[0].lock().unwrap().name(), "foo");
        assert_eq!(units[1].lock().unwrap().name(), "bar");
        assert_eq!(units[1].lock().unwrap().get_dependencies().len(), 1);
        assert_eq!(units[1].lock().unwrap().get_dependencies()[0].lock().unwrap().name(), "foo");
    }
}