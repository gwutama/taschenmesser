mod stop_unit;
mod ping;
mod list_units;

use log::{warn};
use argparse::{ArgumentParser, Store, StoreTrue};
use tsm_ipc::RpcClient;


const BIND_ADDRESS: &str = "ipc:///tmp/tsm-unitman.sock";


// Interesting resource: https://github.com/erickt/rust-zmq/tree/master/examples/zguide
fn main() {
    // init logger
    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug");
    env_logger::init_from_env(env);

    // parse command line arguments
    let mut ping = false;
    let mut list_units = false;
    let mut stop_unit = String::new();

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Taschenmesser Unit Control");
        ap.refer(&mut ping).add_option(&["-p", "--ping"], StoreTrue, "Test connection to unit manager");
        ap.refer(&mut list_units).add_option(&["-l", "--list"], StoreTrue, "List all configured units");
        ap.refer(&mut stop_unit).add_option(&["--stop"], Store, "Stop a unit");
        ap.parse_args_or_exit();
    }

    let rpc_client = match RpcClient::new(String::from(BIND_ADDRESS)) {
        Ok(rpc) => rpc,
        Err(error) => panic!("Failed to create RPC client: {}", error),
    };

    if ping {
        match ping::send_ack_request(rpc_client, String::from("ping")) {
            Ok(ack_response) => println!("Received response: {:?}", ack_response.message),
            Err(error) => warn!("No response received: {}", error),
        };
    } else if list_units {
        match list_units::send_list_units_request(rpc_client) {
            Ok(list_units_response) => {
                list_units::print_units(list_units_response.units);
            },
            Err(error) => warn!("No response received: {}", error),
        };
    } else if !stop_unit.is_empty() {
        match stop_unit::send_stop_unit_request(rpc_client, stop_unit) {
            Ok(response) => println!("Received response: {:?}", response.message),
            Err(error) => warn!("No response received: {}", error),
        };
    } else {
        println!("No command specified. Use --help for more information.");
    }
}
