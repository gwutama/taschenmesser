use tsm_ipc::{RpcClient, tsm_common_rpc, tsm_unitman_rpc};
use protobuf::{Message, Enum};


pub fn send_stop_unit_request(rpc_client: RpcClient, unit_name: String) -> Result<tsm_unitman_rpc::StopUnitResponse, String> {
    let stop_unit_request = build_stop_unit_request(unit_name);

    let response = match rpc_client.send(stop_unit_request) {
        Ok(response) => response,
        Err(error) => {
            return Err(format!("StopUnit = No response received: {}", error));
        },
    };

    return match tsm_unitman_rpc::StopUnitResponse::parse_from_bytes(&response.data) {
        Ok(stop_unit_response) => {
            Ok(stop_unit_response)
        },
        Err(error) => {
            Err(format!("StopUnit = Failed to parse StopUnit response: {}", error))
        }
    };
}


fn build_stop_unit_request(unit_name: String) -> tsm_common_rpc::RpcRequest {
    let mut stop_unit_request = tsm_unitman_rpc::StopUnitRequest::new();
    stop_unit_request.unit_name = unit_name;

    let mut request = tsm_common_rpc::RpcRequest::new();
    request.method = tsm_unitman_rpc::RpcMethod::StopUnit.value();
    request.data = stop_unit_request.write_to_bytes().unwrap();

    request
}