use tsm_ipc::{RpcClient, tsm_common_rpc, tsm_unitman_rpc};
use protobuf::{Message, Enum};


pub fn send_ack_request(rpc_client: RpcClient, message: String) -> Result<tsm_unitman_rpc::AckResponse, String> {
    let ack_request = build_ack_request(message);

    let response = match rpc_client.send(ack_request) {
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
