use std::process::exit;
use argparse::{ArgumentParser, Store};
use log::{error, debug, warn};

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
    let log_level = configuration.get_application().get_log_level();
    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV,
                                                   log_level.to_string());
    env_logger::init_from_env(env);
}


fn init_unit_manager_or_exit(configuration: &config::Configuration) -> unit::UnitManagerRef {
    let manager = unit::UnitManager::new_ref();
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

    return manager;
}


fn main() {
    let params = parse_args_or_exit();

    // init stuffs
    let configuration = init_config_or_exit(params.config_file);
    init_logger(&configuration);
    let manager = init_unit_manager_or_exit(&configuration);

    // start rpc server
    if configuration.get_rpc_server().is_enabled() {
        rpc_server::RpcServer::new(
            manager.clone(),
            configuration.get_rpc_server().get_bind_address()
        ).run_threaded();
    }

    // start unit manager
    match manager.lock() {
        Ok(mut manager) => {
            let handle = manager.run();
            handle.join().expect("Error joining unit manager thread");
        },
        Err(e) => {
            error!("Error acquiring lock: {}", e);
            exit(20);
        }
    };
}
