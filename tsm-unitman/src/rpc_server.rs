use std::sync::{Arc, Mutex};
use log::{debug, warn};
use protobuf::{EnumOrUnknown, Message, Enum};

use tsm_ipc::{tsm_common_rpc, tsm_unitman_rpc};

use crate::unit;


pub struct RpcServer {
    unit_manager: unit::UnitManagerRef,
    bind_address: String,
    rpc_server: tsm_ipc::RpcServer,
}


impl RpcServer {
    pub fn new(unit_manager: unit::UnitManagerRef, bind_address: String) -> Self {
        let request_handler = Arc::new(Mutex::new(
            ResponseHandler::new(unit_manager.clone())
        ));
        let rpc_server = tsm_ipc::RpcServer::new(bind_address.clone(), request_handler);

        Self {
            unit_manager,
            bind_address,
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
                list_units_response.units = self.convert_units_to_proto(&unit_manager.get_units())
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

    /// TODO: Move to rpc_server::Converter
    fn convert_units_to_proto(&self, units: &Vec<unit::UnitRef>) -> Vec<tsm_unitman_rpc::Unit> {
        let mut proto_units = Vec::new();
        for unit in units {
            match self.convert_unit_to_proto(unit) {
                Ok(proto_unit) => proto_units.push(proto_unit),
                Err(error) => warn!("{}", error),
            }
        }
        proto_units
    }

    /// TODO: Move to rpc_server::Converter
    fn convert_unit_to_proto(&self, unit: &unit::UnitRef) -> Result<tsm_unitman_rpc::Unit, String> {
        match unit.try_lock() {
            Ok(mut unit) => {
                let mut proto_unit = tsm_unitman_rpc::Unit::new();

                proto_unit.name = unit.get_name().clone();
                proto_unit.executable = unit.get_executable().clone();
                proto_unit.arguments = unit.get_arguments().clone();
                proto_unit.restart_policy = EnumOrUnknown::from_i32(unit.get_restart_policy().clone() as i32);
                proto_unit.uid = unit.get_uid() as i32;
                proto_unit.gid = unit.get_gid() as i32;
                proto_unit.enabled = unit.is_enabled();
                proto_unit.process_probe_state = EnumOrUnknown::from_i32(unit.get_process_probe_state().clone() as i32);
                proto_unit.liveness_probe_state = EnumOrUnknown::from_i32(unit.get_liveness_probe_state().clone() as i32);

                match unit.get_pid() {
                    Some(pid) => proto_unit.pid = pid as i32,
                    None => proto_unit.pid = -1,
                }

                Ok(proto_unit)
            },
            Err(_) => {
                return Err("Failed to lock unit".to_string());
            },
        }
    }
}