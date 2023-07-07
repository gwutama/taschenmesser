use tsm_ipc::{tsm_common_rpc, tsm_unitman_rpc};
use protobuf::{Message, Enum};
use log::{debug, warn};


pub fn handle_ping(request: tsm_common_rpc::RpcRequest) -> tsm_common_rpc::RpcResponse {
    let mut rpc_response = tsm_common_rpc::RpcResponse::new();

    let ping_request: tsm_unitman_rpc::PingRequest = match Message::parse_from_bytes(&request.data) {
        Ok(request) => {
            rpc_response.method = tsm_unitman_rpc::RpcMethod::Ping.value();
            request
        },
        Err(error) => {
            warn!("Failed to parse ping request: {}", error);
            rpc_response.method = tsm_unitman_rpc::RpcMethod::Ping.value();
            rpc_response.status = false;
            rpc_response.error = format!("Failed to parse ping request: {}", error);
            return rpc_response;
        },
    };

    debug!("Received ping request: {}", ping_request.message);

    let mut ping_response = tsm_unitman_rpc::PingResponse::new();
    ping_response.message = "pong".to_string();

    match ping_response.write_to_bytes() {
        Ok(bytes) => {
            rpc_response.status = true;
            rpc_response.data = bytes;
        },
        Err(error) => {
            rpc_response.status = false;
            rpc_response.error = format!("Failed to serialize ping response: {}", error);
        },
    }

    return rpc_response;
}