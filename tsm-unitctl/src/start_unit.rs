use tsm_ipc::{RpcClient, tsm_common_rpc, tsm_unitman_rpc};
use protobuf::{Message, Enum};


pub fn send_start_unit_request(rpc_client: RpcClient, unit_name: String) -> Result<tsm_unitman_rpc::StartUnitResponse, String> {
    let start_unit_request = build_start_unit_request(unit_name);

    let response = match rpc_client.send(start_unit_request) {
        Ok(response) => response,
        Err(error) =>  return Err(format!("{}", error)),
    };

    if !response.status {
        return Err(format!("{}", response.error));
    }

    return match tsm_unitman_rpc::StartUnitResponse::parse_from_bytes(&response.data) {
        Ok(start_unit_response) => Ok(start_unit_response),
        Err(error) => Err(format!("Failed to parse response: {}", error)),
    };
}


fn build_start_unit_request(unit_name: String) -> tsm_common_rpc::RpcRequest {
    let mut start_unit_request = tsm_unitman_rpc::StartUnitRequest::new();
    start_unit_request.unit_name = unit_name;

    let mut request = tsm_common_rpc::RpcRequest::new();
    request.method = tsm_unitman_rpc::RpcMethod::StartUnit.value();
    request.data = start_unit_request.write_to_bytes().unwrap();

    request
}