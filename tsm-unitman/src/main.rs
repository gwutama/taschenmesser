use std::process::exit;
use argparse::{ArgumentParser, Store};
use log::error;

mod config;
mod unit;
mod rpc_server;


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


fn init_config_or_exit(config_file: String) -> config::Configuration {
    match config::Configuration::from_file(config_file) {
        Ok(configuration) => {
            configuration
        },
        Err(e) => {
            error!("Error: {}", e);
            exit(10);
        }
    }
}


fn init_logger(configuration: &config::Configuration) {
    let mut log_level = configuration.application.get_log_level();
    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV,
                                                   log_level.to_string());
    env_logger::init_from_env(env);
}


fn init_unit_manager_or_exit(configuration: &config::Configuration) -> unit::ManagerRef {
    let manager = unit::Manager::new_ref();
    let units = configuration.build_units();

    match manager.lock() {
        Ok(mut manager) => {
            for unit in units {
                manager.add_unit(unit);
            }
        },
        Err(e) => {
            error!("Error acquiring lock: {}", e);
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
                error!("Error acquiring lock: {}", e);
                exit(20);
            }
        }
    }).expect("Error setting Ctrl-C handler");

    return manager;
}


fn init_rpc_server_or_exit(
    configuration: &config::Configuration,
    unit_manager: unit::ManagerRef,
    ) -> Option<rpc_server::RpcServer> {
    if configuration.rpc_server.is_enabled() {
        let bind_address = configuration.rpc_server.get_bind_address();
        return Some(rpc_server::RpcServer::new(unit_manager.clone(),
                                               bind_address.clone()));
    }

    None
}


fn main() {
    let params = parse_args_or_exit();
    let configuration = init_config_or_exit(params.config_file);

    init_logger(&configuration);

    let manager = init_unit_manager_or_exit(&configuration);

    match init_rpc_server_or_exit(&configuration, manager.clone()) {
        Some(rpc_server) => {
            rpc_server.run_threaded();
        },
        None => {}
    }

    unit::Runner::run(manager.clone());
}
