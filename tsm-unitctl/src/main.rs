use tsm_ipc::{tsm_unitman_rpc, tsm_common_rpc, RpcClient};
use protobuf::{Message, Enum};
use log::{debug, warn};

const BIND_ADDRESS: &str = "ipc:///tmp/tsm-unitman.sock";

// Interesting resource: https://github.com/erickt/rust-zmq/tree/master/examples/zguide
fn main() {
    // init logger
    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug");
    env_logger::init_from_env(env);

    match send_ack_request(String::from("ping")) {
        Ok(ack_response) => debug!("Ack = Received response: {:?}", ack_response),
        Err(error) => warn!("Ack = No response received: {}", error),
    };

    match send_list_units_request() {
        Ok(list_units_response) => debug!("ListUnits = Received response: {:?}", list_units_response.units),
        Err(error) => warn!("ListUnits = No response received: {}", error),
    };
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
