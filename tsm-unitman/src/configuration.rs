use std::collections::HashMap;
use std::fs;
use serde::Deserialize;
use log::{error, warn};
use users::{get_current_gid, get_current_uid, get_group_by_name, get_user_by_name};

use crate::unit::unit::{Unit, UnitRef};
use crate::log_level::LogLevel;
use crate::unit::restart_policy::RestartPolicy;


#[derive(Deserialize, Debug)]
pub struct ApplicationConfiguration {
    pub log_level: Option<LogLevel>,
}


#[derive(Deserialize, Debug)]
pub struct UnitConfiguration {
    name: String,
    executable: String,
    arguments: Vec<String>,
    dependencies: Vec<String>,
    restart_policy: RestartPolicy,
    user: Option<String>,
    group: Option<String>,
    enabled: bool,
}


impl UnitConfiguration {
    pub fn build_ref(&self) -> UnitRef {
        return Unit::new_ref(
            self.name.clone(),
            self.executable.clone(),
            self.arguments.clone(),
            self.restart_policy.clone(),
            self.determine_uid(),
            self.determine_gid(),
            self.enabled.clone());
    }

    /// If user is valid, return its uid
    /// Otherwise, return own uid
    fn determine_uid(&self) -> u32 {
        return match &self.user {
            Some(user) => {
                match get_user_by_name(user) {
                    Some(user) => user.uid(),
                    None => get_current_uid(),
                }
            },
            None => {
                get_current_uid()
            }
        }
    }

    /// If group is valid, return its gid
    /// Otherwise, return own gid
    fn determine_gid(&self) -> u32 {
        return match &self.group {
            Some(group) => {
                match get_group_by_name(group) {
                    Some(group) => group.gid(),
                    None => get_current_gid(),
                }
            },
            None => {
                get_current_gid()
            }
        }
    }
}


#[derive(Deserialize, Debug)]
pub struct Configuration {
    pub application: ApplicationConfiguration,
    units: Vec<UnitConfiguration>,
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
            Ok(configuration) => {
                Ok(configuration)
            },
            Err(error) => {
                Err(format!("Error parsing configuration file: {}", error))
            }
        }
    }

    pub fn build_units(&self) -> Vec<UnitRef> {
        let mut units = Vec::new();

        // In order to build the dependencies, we need to build all units first and push them
        // into a hash map. Then, we can iterate over the hash map and build the dependencies.
        let mut unit_map: HashMap<String, UnitRef> = HashMap::new();

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
            let unit_ref = match unit_map.get(&unit_configuration.name) {
                Some(unit_ref) => unit_ref,
                None => {
                    warn!("Unit {} not found in unit map", unit_configuration.name);
                    continue;
                }
            };

            for dependency_name in &unit_configuration.dependencies {
                let dependency_unit_ref = match unit_map.get(dependency_name) {
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

    fn sample_working_all_conf() -> String {
        return String::from(
            r#"
                [application]
                log_level = "debug"

                [[units]]
                name = "foo"
                executable = "ls"
                arguments = [ "-l", "-a" ]
                dependencies = []
                restart_policy = "always"
                user = ""
                group = ""
                enabled = true

                [[units]]
                name = "bar"
                executable = "ps"
                arguments = [ "-aux" ]
                dependencies = [ "foo" ]
                restart_policy = "never"
                user = ""
                group = ""
                enabled = true
            "#,
        );
    }

    fn sample_working_mandatory_only_conf() -> String {
        return String::from(
            r#"
                [application]

                [[units]]
                name = "foo"
                executable = "ls"
                arguments = [ "-l", "-a" ]
                dependencies = []
                restart_policy = "always"
                enabled = true

                [[units]]
                name = "bar"
                executable = "ps"
                arguments = [ "-aux" ]
                dependencies = [ "foo" ]
                restart_policy = "never"
                user = ""
                group = ""
                enabled = true
            "#,
        );
    }


    #[test]
    fn from_string_should_work() {
        let content= sample_working_all_conf();
        let configuration = Configuration::from_string(content).unwrap();

        assert_eq!(configuration.application.log_level.unwrap(), LogLevel::Debug);
        assert_eq!(configuration.units.len(), 2);
    }

    #[test]
    fn from_string_when_missing_optional_keys_should_work() {
        let content= sample_working_mandatory_only_conf();
        let configuration = Configuration::from_string(content).unwrap();

        assert_eq!(configuration.application.log_level, None);
    }

    #[test]
    fn from_file_should_work() {
        let file = String::from("resources/tsm-unitman.toml");
        let configuration = Configuration::from_file(file).unwrap();

        assert_eq!(configuration.application.log_level.unwrap(), LogLevel::Debug);
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
        let content= sample_working_all_conf();
        let configuration = Configuration::from_string(content).unwrap();

        let units = configuration.build_units();

        assert_eq!(units.len(), 2);
        assert_eq!(units[0].lock().unwrap().name(), "foo");
        assert_eq!(units[1].lock().unwrap().name(), "bar");
        assert_eq!(units[1].lock().unwrap().dependencies().len(), 1);
        assert_eq!(units[1].lock().unwrap().dependencies()[0].lock().unwrap().name(), "foo");
    }
}