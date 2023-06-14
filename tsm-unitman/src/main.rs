use std::process::exit;
use argparse::{ArgumentParser, StoreTrue, Store};
use ctrlc::set_handler;

mod unit;
mod configuration;

use configuration::Configuration;
use unit::manager::{Manager, ManagerRef};
use unit::runner::Runner;


struct CommandLineParameters {
    config_file: String,
}


fn parse_args_or_exit() -> CommandLineParameters {
    let mut config_file = String::new();

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Taschenmesser Unit Manager");
        ap.refer(&mut config_file).add_option(&["-c", "--config"], Store, "Configuration file");
        ap.parse_args_or_exit();
    }

    CommandLineParameters {
        config_file,
    }
}


fn init_config_or_exit(config_file: String) -> Configuration {
    match Configuration::from_file(config_file) {
        Ok(configuration) => {
            configuration
        },
        Err(e) => {
            println!("Error: {}", e);
            exit(10);
        }
    }
}


fn init_unit_manager_or_exit(configuration: Configuration) -> ManagerRef {
    let manager = Manager::new_ref();
    let units = configuration.build_units();

    match manager.lock() {
        Ok(mut manager) => {
            for unit in units {
                manager.add_unit(unit);
            }
        },
        Err(e) => {
            println!("Error acquiring lock: {}", e);
            exit(20);
        }
    }

    let manager_clone = manager.clone();
    ctrlc::set_handler(move || {
        match manager_clone.lock() {
            Ok(mut manager) => {
                manager.request_stop();
            },
            Err(e) => {
                println!("Error acquiring lock: {}", e);
                exit(20);
            }
        }
    }).expect("Error setting Ctrl-C handler");

    return manager;
}


fn main() {
    let params = parse_args_or_exit();
    let configuration = init_config_or_exit(params.config_file);
    let manager = init_unit_manager_or_exit(configuration);
    Runner::run(manager.clone());
}
