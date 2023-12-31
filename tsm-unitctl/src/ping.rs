use tsm_ipc::{RpcClient, tsm_common_rpc, tsm_unitman_rpc};
use protobuf::{Message, Enum};


pub fn send_ping_request(rpc_client: RpcClient, message: String) -> Result<tsm_unitman_rpc::PingResponse, String> {
    let ping_request = build_ping_request(message);

    let response = match rpc_client.send(ping_request) {
        Ok(response) => response,
        Err(error) =>  return Err(format!("{}", error)),
    };

    if !response.status {
        return Err(format!("{}", response.error));
    }

    return match tsm_unitman_rpc::PingResponse::parse_from_bytes(&response.data) {
        Ok(ping_response) => Ok(ping_response),
        Err(error) => Err(format!("Failed to parse response: {}", error)),
    };
}


fn build_ping_request(message: String) -> tsm_common_rpc::RpcRequest {
    let mut ping_request = tsm_unitman_rpc::PingRequest::new();
    ping_request.message = message;

    let mut request = tsm_common_rpc::RpcRequest::new();
    request.method = tsm_unitman_rpc::RpcMethod::Ping.value();
    request.data = ping_request.write_to_bytes().unwrap();

    request
}
