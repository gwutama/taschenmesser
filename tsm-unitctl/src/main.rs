use tsm_ipc::{tsm_unitman_rpc, tsm_common_rpc, RpcClient};
use protobuf::{Message, Enum};
use log::{debug, warn};
use argparse::{ArgumentParser, StoreTrue};
use tabled::{builder::Builder, settings::Style};

const BIND_ADDRESS: &str = "ipc:///tmp/tsm-unitman.sock";

// Interesting resource: https://github.com/erickt/rust-zmq/tree/master/examples/zguide
fn main() {
    // init logger
    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug");
    env_logger::init_from_env(env);

    // parse command line arguments
    let mut ping = false;
    let mut list_units = false;

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Taschenmesser Unit Control");
        ap.refer(&mut ping).add_option(&["-p", "--ping"], StoreTrue, "Test connection to unit manager");
        ap.refer(&mut list_units).add_option(&["-l", "--list"], StoreTrue, "List units");
        ap.parse_args_or_exit();
    }

    if ping {
        match send_ack_request(String::from("ping")) {
            Ok(ack_response) => println!("Received response: {:?}", ack_response.message),
            Err(error) => warn!("No response received: {}", error),
        };
    } else if list_units {
        match send_list_units_request() {
            Ok(list_units_response) => {
                print_units(list_units_response.units);
            },
            Err(error) => warn!("No response received: {}", error),
        };
    } else {
        println!("No command specified. Use --help for more information.");
    }
}


fn print_units(units: Vec<tsm_unitman_rpc::Unit>) {
    let mut builder = Builder::new();
    builder.set_header(vec!["UNIT NAME", "IS ENABLED", "RESTART POLICY", "RUN STATE", "PROBE STATE", "COMMAND"]);

    for unit in units {
        let enabled = match unit.enabled {
            true => String::from("Enabled"),
            false => String::from("Disabled"),
        };

        let restart_policy = match tsm_unitman_rpc::unit::RestartPolicy::from_i32(unit.restart_policy.value()) {
            Some(policy) => match policy {
                tsm_unitman_rpc::unit::RestartPolicy::Always => String::from("Always"),
                tsm_unitman_rpc::unit::RestartPolicy::Never => String::from("Never"),
            },
            None => String::from("Unknown"),
        };

        let probe_state = match tsm_unitman_rpc::unit::ProbeState::from_i32(unit.probe_state.value()) {
            Some(state) => match state {
                tsm_unitman_rpc::unit::ProbeState::Undefined => String::from("Undefined"),
                tsm_unitman_rpc::unit::ProbeState::Alive => String::from("Alive"),
                tsm_unitman_rpc::unit::ProbeState::Dead => String::from("Dead"),
            },
            None => String::from("Unknown"),
        };

        let command = format!("{} {}", unit.executable, unit.arguments.join(" "));

        let run_state: String = match unit.pid {
            n if n > 0 => String::from(format!("Running (pid={})", n)),
            _ => String::from("Stopped"),
        };

        builder.push_record([unit.name, enabled, restart_policy, run_state, probe_state, command]);
    }

    let mut table = builder.build();
    table.with(Style::modern());

    let table = table.to_string();
    println!("{}", table);
}


fn send_ack_request(message: String) -> Result<tsm_unitman_rpc::AckResponse, String>{
    let rpc = match RpcClient::new(String::from(BIND_ADDRESS)) {
        Ok(rpc) => rpc,
        Err(error) => panic!("Failed to create RPC client: {}", error),
    };

    let ack_request = build_ack_request(message);

    let response = match rpc.send(ack_request) {
        Ok(response) => response,
        Err(error) => {
            return Err(format!("Ack = No response received: {}", error));
        },
    };

    // TODO: parse response.method, response.status and response.error too!
    return match tsm_unitman_rpc::AckResponse::parse_from_bytes(&response.data) {
        Ok(ack_response) => {
            Ok(ack_response)
        },
        Err(error) => {
            Err(format!("Failed to parse Ack response: {}", error))
        }
    };
}


fn build_ack_request(message: String) -> tsm_common_rpc::RpcRequest {
    let mut ack_request = tsm_unitman_rpc::AckRequest::new();
    ack_request.message = message;

    let mut request = tsm_common_rpc::RpcRequest::new();
    request.method = tsm_unitman_rpc::RpcMethod::Ack.value();
    request.data = ack_request.write_to_bytes().unwrap();

    request
}


fn send_list_units_request() -> Result<tsm_unitman_rpc::ListUnitsResponse, String> {
    let rpc = match RpcClient::new(String::from(BIND_ADDRESS)) {
        Ok(rpc) => rpc,
        Err(error) => panic!("Failed to create RPC client: {}", error),
    };

    let unit_list_request = build_list_units_request();

    let response = match rpc.send(unit_list_request) {
        Ok(response) => response,
        Err(error) => {
            return Err(format!("ListUnits = No response received: {}", error));
        },
    };

    // TODO: parse response.method, response.status and response.error too!
    return match tsm_unitman_rpc::ListUnitsResponse::parse_from_bytes(&response.data) {
        Ok(list_units_response) => {
            Ok(list_units_response)
        },
        Err(error) => {
            Err(format!("ListUnits = Failed to parse ListUnits response: {}", error))
        }
    };
}


fn build_list_units_request() -> tsm_common_rpc::RpcRequest {
    let mut request = tsm_common_rpc::RpcRequest::new();
    request.method = tsm_unitman_rpc::RpcMethod::ListUnits.value();

    request
}
