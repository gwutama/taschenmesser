use tsm_ipc::{tsm_common_rpc, tsm_unitman_rpc};
use protobuf::{Message, Enum};
use log::{debug, warn};

use crate::unit;
use crate::rpc::converters;


pub fn handle_list_units(request: tsm_common_rpc::RpcRequest, unit_manager: unit::UnitManagerRef) -> tsm_common_rpc::RpcResponse {
    let mut rpc_response = tsm_common_rpc::RpcResponse::new();

    let _list_units_request: tsm_unitman_rpc::ListUnitsRequest = match Message::parse_from_bytes(&request.data) {
        Ok(request) => {
            rpc_response.method = tsm_unitman_rpc::RpcMethod::ListUnits.value();
            request
        },
        Err(error) => {
            warn!("Failed to parse list units request: {}", error);
            rpc_response.method = tsm_unitman_rpc::RpcMethod::ListUnits.value();
            rpc_response.status = false;
            rpc_response.error = format!("Failed to parse list units request: {}", error);
            return rpc_response;
        },
    };

    debug!("Received list units request");

    let mut list_units_response = tsm_unitman_rpc::ListUnitsResponse::new();

    match unit_manager.try_lock() {
        Ok(unit_manager) => {
            list_units_response.units = converters::convert_units_to_proto(&unit_manager.get_units())
        },
        Err(error) => {
            warn!("Failed to lock unit manager: {}", error);
            rpc_response.status = false;
            rpc_response.error = format!("Failed to lock unit manager: {}", error);
            return rpc_response;
        },
    }

    match list_units_response.write_to_bytes() {
        Ok(bytes) => {
            rpc_response.status = true;
            rpc_response.data = bytes;
        },
        Err(error) => {
            rpc_response.status = false;
            rpc_response.error = format!("Failed to serialize list units response: {}", error);
        },
    }

    return rpc_response;
}