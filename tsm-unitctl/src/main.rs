mod ping;
mod list_units;
mod start_unit;
mod stop_unit;

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
    let mut start_unit = String::new();

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Taschenmesser Unit Control");
        ap.refer(&mut ping).add_option(&["--ping"], StoreTrue, "Test connection to unit manager");
        ap.refer(&mut list_units).add_option(&["--list"], StoreTrue, "List all configured units");
        ap.refer(&mut stop_unit).add_option(&["--stop"], Store, "Stop a unit");
        ap.refer(&mut start_unit).add_option(&["--start"], Store, "Start a unit");
        ap.parse_args_or_exit();
    }

    let rpc_client = match RpcClient::new(String::from(BIND_ADDRESS)) {
        Ok(rpc) => rpc,
        Err(error) => panic!("Failed to create RPC client: {}", error),
    };

    if ping {
        match ping::send_ping_request(rpc_client, String::from("ping")) {
            Ok(response) => println!("{}", response.message),
            Err(error) => println!("{}", error),
        };
    } else if list_units {
        match list_units::send_list_units_request(rpc_client) {
            Ok(list_units_response) => list_units::print_units(list_units_response.units),
            Err(error) => println!("{}", error),
        };
    } else if !stop_unit.is_empty() {
        match stop_unit::send_stop_unit_request(rpc_client, stop_unit) {
            Ok(response) => println!("{}", response.message),
            Err(error) => println!("{}", error),
        };
    } else if !start_unit.is_empty() {
        match start_unit::send_start_unit_request(rpc_client, start_unit) {
            Ok(response) => println!("{}", response.message),
            Err(error) => println!("{}", error),
        };
    } else {
        println!("No command specified. Use --help for more information.");
    }
}
