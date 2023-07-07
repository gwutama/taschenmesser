use std::sync::{Arc, Mutex};
use log::{debug, warn};
use protobuf::{Message, Enum};

use tsm_ipc::{tsm_common_rpc, tsm_unitman_rpc};
use crate::rpc::converters;

use crate::unit;


pub struct RpcServer {
    rpc_server: tsm_ipc::RpcServer,
}


impl RpcServer {
    pub fn new(unit_manager: unit::UnitManagerRef, bind_address: String) -> Self {
        let request_handler = Arc::new(Mutex::new(
            ResponseHandler::new(unit_manager.clone())
        ));
        let rpc_server = tsm_ipc::RpcServer::new(bind_address.clone(), request_handler);

        Self {
            rpc_server,
        }
    }

    pub fn run_threaded(self) {
        self.rpc_server.run_threaded();
    }
}


struct ResponseHandler {
    unit_manager: unit::UnitManagerRef,
}


impl tsm_ipc::RpcRequestHandler for ResponseHandler {
    fn handle_request(&self, request: tsm_common_rpc::RpcRequest) -> tsm_common_rpc::RpcResponse {
        let request_method= match tsm_unitman_rpc::RpcMethod::from_i32(request.method) {
            Some(method) => method,
            None => tsm_unitman_rpc::RpcMethod::Unknown,
        };

        // Handle request based on method
        match request_method {
            tsm_unitman_rpc::RpcMethod::Ack => self.handle_ack(request),
            tsm_unitman_rpc::RpcMethod::ListUnits => self.handle_list_units(request),
            tsm_unitman_rpc::RpcMethod::StopUnit => self.handle_stop_unit(request),
            _ => self.handle_unknown(),
        }
    }
}


impl ResponseHandler {
    pub fn new(unit_manager: unit::UnitManagerRef) -> Self {
        Self {
            unit_manager,
        }
    }

    fn handle_unknown(&self) -> tsm_common_rpc::RpcResponse {
        warn!("Cannot handle unknown method");
        let mut rpc_response = tsm_common_rpc::RpcResponse::new();
        rpc_response.method = tsm_unitman_rpc::RpcMethod::Unknown.value();
        rpc_response.status = false;
        rpc_response.error = format!("Unknown method");
        return rpc_response;
    }

    fn handle_ack(&self, request: tsm_common_rpc::RpcRequest) -> tsm_common_rpc::RpcResponse {
        let mut rpc_response = tsm_common_rpc::RpcResponse::new();

        let ack_request: tsm_unitman_rpc::AckRequest = match Message::parse_from_bytes(&request.data) {
            Ok(request) => {
                rpc_response.method = tsm_unitman_rpc::RpcMethod::Ack.value();
                request
            },
            Err(error) => {
                warn!("Failed to parse ack request: {}", error);
                rpc_response.method = tsm_unitman_rpc::RpcMethod::Ack.value();
                rpc_response.status = false;
                rpc_response.error = format!("Failed to parse ack request: {}", error);
                return rpc_response;
            },
        };

        debug!("Received ack request: {}", ack_request.message);

        let mut ack_response = tsm_unitman_rpc::AckResponse::new();
        ack_response.message = "pong".to_string();

        match ack_response.write_to_bytes() {
            Ok(bytes) => {
                rpc_response.status = true;
                rpc_response.data = bytes;
            },
            Err(error) => {
                rpc_response.status = false;
                rpc_response.error = format!("Failed to serialize ack response: {}", error);
            },
        }

        return rpc_response;
    }

    fn handle_list_units(&self, request: tsm_common_rpc::RpcRequest) -> tsm_common_rpc::RpcResponse {
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

        match self.unit_manager.try_lock() {
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

    fn handle_stop_unit(&self, request: tsm_common_rpc::RpcRequest) -> tsm_common_rpc::RpcResponse {
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

        match self.unit_manager.try_lock() {
            Ok(unit_manager) => {
                match unit_manager.stop_unit(String::from(stop_unit_request.unit_name)) {
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
}