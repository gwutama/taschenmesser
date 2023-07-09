use std::fs;
use serde::Deserialize;

use crate::config::{Application, Server};


#[derive(Deserialize, Debug)]
pub struct Configuration {
    application: Application,
    server: Server,
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

    pub fn get_application(&self) -> &Application {
        return &self.application;
    }

    pub fn get_server(&self) -> &Server {
        return &self.server;
    }
}
