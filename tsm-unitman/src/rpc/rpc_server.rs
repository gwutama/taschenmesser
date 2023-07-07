use std::sync::{Arc, Mutex};
use log::{debug, warn};
use protobuf::{Message, Enum};

use tsm_ipc::{tsm_common_rpc, tsm_unitman_rpc};
use crate::rpc::{stop_unit, start_unit, list_units, ping};

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
            tsm_unitman_rpc::RpcMethod::Ping => ping::handle_ping(request),
            tsm_unitman_rpc::RpcMethod::ListUnits => list_units::handle_list_units(request, self.unit_manager.clone()),
            tsm_unitman_rpc::RpcMethod::StartUnit => start_unit::handle_start_unit(request, self.unit_manager.clone()),
            tsm_unitman_rpc::RpcMethod::StopUnit => stop_unit::handle_stop_unit(request, self.unit_manager.clone()),
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
}