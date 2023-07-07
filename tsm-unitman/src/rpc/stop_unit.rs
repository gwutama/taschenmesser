use tsm_ipc::{tsm_common_rpc, tsm_unitman_rpc};
use protobuf::{Message, Enum};
use log::{debug, warn};

use crate::unit;


pub fn handle_stop_unit(request: tsm_common_rpc::RpcRequest, unit_manager: unit::UnitManagerRef) -> tsm_common_rpc::RpcResponse {
    let mut rpc_response = tsm_common_rpc::RpcResponse::new();

    let stop_unit_request: tsm_unitman_rpc::StopUnitRequest = match Message::parse_from_bytes(&request.data) {
        Ok(request) => {
            rpc_response.method = tsm_unitman_rpc::RpcMethod::StopUnit.value();
            request
        },
        Err(error) => {
            warn!("Failed to parse stop unit request: {}", error);
            rpc_response.method = tsm_unitman_rpc::RpcMethod::StopUnit.value();
            rpc_response.status = false;
            rpc_response.error = format!("Failed to parse stop unit request: {}", error);
            return rpc_response;
        },
    };

    debug!("Received stop unit request: {}", stop_unit_request.unit_name);

    match unit_manager.try_lock() {
        Ok(unit_manager) => {
            match unit_manager.stop_unit(String::from(stop_unit_request.unit_name), false) {
                Ok(_) => {
                    rpc_response.status = true;
                },
                Err(error) => {
                    rpc_response.status = false;
                    rpc_response.error = format!("Failed to stop unit: {}", error);
                },
            }
        },
        Err(error) => {
            warn!("Failed to lock unit manager: {}", error);
            rpc_response.status = false;
            rpc_response.error = format!("Failed to lock unit manager: {}", error);
        },
    }

    return rpc_response;
}