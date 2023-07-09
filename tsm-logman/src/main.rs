use std::process::exit;
use std::net::UdpSocket;
use std::thread;
use log::{error};
use argparse::{ArgumentParser, Store};

mod config;


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


fn start_listen(host: String, port: u16) {
    let addr = format!("{}:{}", host, port);
    let socket = UdpSocket::bind(addr.clone()).expect("couldn't bind to address");
    let mut buf = [0; 4096];

    println!("Listening on {}", addr.clone());

    loop {
        match socket.try_clone() {
            Ok(socket) => {
                match socket.recv_from(&mut buf) {
                    Ok((amt, src)) => {
                        thread::spawn(move || {
                            println!("Received {} bytes from {}", amt, src);
                            let buffer = &mut buf[..amt];
                            buffer.reverse();
                            socket.send_to(buffer, &src).expect("couldn't send data");
                        });
                    }
                    Err(e) => {
                        println!("couldn't receive a datagram: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("couldn't clone socket: {}", e);
            }
        }
    }
}


fn main() {
    let params = parse_args_or_exit();

    // init stuffs
    let configuration = init_config_or_exit(params.config_file);
    init_logger(&configuration);

    // start server
    start_listen(configuration.get_server().get_host(),
                 configuration.get_server().get_port());
}
