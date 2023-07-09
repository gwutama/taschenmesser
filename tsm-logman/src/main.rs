use std::error::Error;
use std::process::exit;
use log::{error};
use tokio::net::UdpSocket;
use argparse::{ArgumentParser, Store};

mod config;
mod server;


struct CommandLineParameters {
    config_file: String,
}


fn parse_args_or_exit() -> CommandLineParameters {
    let mut config_file = String::new();

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Taschenmesser Log Manager");
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


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let params = parse_args_or_exit();

    // init stuffs
    let configuration = init_config_or_exit(params.config_file);
    init_logger(&configuration);

    // start server
    let host = configuration.get_server().get_host();
    let port = configuration.get_server().get_port();
    let addr = format!("{}:{}", host, port);
    let socket = UdpSocket::bind(addr.clone()).await?;

    println!("Listening on: {}", socket.local_addr()?);

    let server = server::UdpServer::new(socket);

    // This starts the server task.
    server.run().await?;

    Ok(())
}
