use tsm_ipc::{RpcClient, tsm_common_rpc, tsm_unitman_rpc};
use protobuf::{Message, Enum};
use tabled::{builder::Builder, settings::Style};


pub fn send_list_units_request(rpc_client: RpcClient) -> Result<tsm_unitman_rpc::ListUnitsResponse, String> {
    let unit_list_request = build_list_units_request();

    let response = match rpc_client.send(unit_list_request) {
        Ok(response) => response,
        Err(error) =>  return Err(format!("{}", error)),
    };

    if !response.status {
        return Err(format!("{}", response.error));
    }

    return match tsm_unitman_rpc::ListUnitsResponse::parse_from_bytes(&response.data) {
        Ok(list_units_response) => Ok(list_units_response),
        Err(error) => Err(format!("Failed to parse response: {}", error)),
    };
}


fn build_list_units_request() -> tsm_common_rpc::RpcRequest {
    let mut request = tsm_common_rpc::RpcRequest::new();
    request.method = tsm_unitman_rpc::RpcMethod::ListUnits.value();

    request
}


pub fn print_units(units: Vec<tsm_unitman_rpc::Unit>) {
    let mut builder = Builder::new();
    builder.set_header(vec!["UNIT NAME", "IS ENABLED", "RESTART POLICY", "RUN STATE", "PROCESS STATE", "LIVENESS STATE", "COMMAND"]);

    for unit in units {
        let enabled = match unit.enabled {
            true => String::from("Enabled"),
            false => String::from("Disabled"),
        };

        let restart_policy = match tsm_unitman_rpc::unit::RestartPolicy::from_i32(unit.restart_policy.value()) {
            Some(policy) => match policy {
                tsm_unitman_rpc::unit::RestartPolicy::Always => String::from("Always"),
                tsm_unitman_rpc::unit::RestartPolicy::Never => String::from("Never"),
                tsm_unitman_rpc::unit::RestartPolicy::DisabledTemporarily => String::from("Disabled*"),
            },
            None => String::from("Unknown"),
        };

        let process_probe_state = match tsm_unitman_rpc::unit::ProbeState::from_i32(unit.process_probe_state.value()) {
            Some(state) => match state {
                tsm_unitman_rpc::unit::ProbeState::Undefined => String::from("Undefined"),
                tsm_unitman_rpc::unit::ProbeState::Alive => String::from("Alive"),
                tsm_unitman_rpc::unit::ProbeState::Dead => String::from("Dead"),
            },
            None => String::from("Unknown"),
        };

        let liveness_probe_state = match tsm_unitman_rpc::unit::ProbeState::from_i32(unit.liveness_probe_state.value()) {
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

        builder.push_record([unit.name, enabled, restart_policy, run_state, process_probe_state, liveness_probe_state, command]);
    }

    let mut table = builder.build();
    table.with(Style::modern());

    let table = table.to_string();
    println!("{}", table);
}