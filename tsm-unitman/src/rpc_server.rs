use std::thread;
use log::{debug, warn};
use protobuf::{EnumOrUnknown, Message};

use unit::ManagerRef;

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
use tsm_unitman_rpc::{RpcRequest, RpcResponse, RpcMethod,
                      AckRequest, AckResponse,
                      ListUnitsRequest, ListUnitsResponse};
use crate::unit;


pub struct RpcServer {
    unit_manager: ManagerRef,
    bind_address: String,
}


impl RpcServer {
    /// TODO: Move to library
    pub fn new(unit_manager: ManagerRef, bind_address: String) -> Self {
        Self {
            unit_manager,
            bind_address,
        }
    }

    /// TODO: Move to library
    pub fn run_threaded(self) -> thread::JoinHandle<()> {
        debug!("Spawning thread");
        thread::spawn(move || {
            self.run();
        })
    }

    /// TODO: Move to library
    fn run(self) {
        let context = zmq::Context::new();
        let responder = match context.socket(zmq::REP) {
            Ok(socket) => socket,
            Err(error) => {
                warn!("Failed to create socket: {}", error);
                return;
            },
        };

        assert!(responder.bind(self.bind_address.as_str()).is_ok());

        loop {
            // https://github.com/pronebird/node-rust-zeromq/blob/master/server/src/main.rs
            let request: Option<RpcRequest> = match responder.recv_bytes(0) {
                Ok(bytes) => Some(Message::parse_from_bytes(&bytes).unwrap()),
                Err(_) => None,
            };

            // handle request
            let response: Option<RpcResponse> = match request {
                Some(request) => {
                    debug!("Received request: {:?}", request);
                    Some(self.handle_request(request))
                },
                None => None,
            };

            match response {
                Some(response) => {
                    debug!("Sending response: {:?}", response);

                    let message = match response.write_to_bytes() {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            warn!("Failed to serialize response: {}", error);
                            continue;
                        },
                    };

                    match responder.send(&message, 0) {
                        Ok(_) => (),
                        Err(error) => {
                            warn!("Failed to send response: {}", error);
                            continue;
                        },
                    };
                },
                None => (),
            }
        }
    }

    /// TODO: Move to library. Use trait.
    fn handle_request(&self, request: RpcRequest) -> RpcResponse {
        // Handle request based on method
        match request.method.enum_value_or_default() {
            RpcMethod::Ack => self.handle_ack(request),
            RpcMethod::ListUnits => self.handle_list_units(request),
            _ => self.handle_unknown(),
        }
    }

    fn handle_unknown(&self) -> RpcResponse {
        warn!("Cannot handle unknown method");
        let mut rpc_response = RpcResponse::new();
        rpc_response.method = EnumOrUnknown::from(RpcMethod::Unknown);
        rpc_response.status = false;
        rpc_response.error = format!("Unknown method");
        return rpc_response;
    }

    fn handle_ack(&self, request: RpcRequest) -> RpcResponse {
        let mut rpc_response = RpcResponse::new();

        let ack_request: AckRequest = match Message::parse_from_bytes(&request.data) {
            Ok(request) => {
                rpc_response.method = EnumOrUnknown::from(RpcMethod::Ack);
                request
            },
            Err(error) => {
                warn!("Failed to parse ack request: {}", error);
                rpc_response.method = EnumOrUnknown::from(RpcMethod::Ack);
                rpc_response.status = false;
                rpc_response.error = format!("Failed to parse ack request: {}", error);
                return rpc_response;
            },
        };

        debug!("Received ack request: {}", ack_request.message);

        let mut ack_response = AckResponse::new();
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

    fn handle_list_units(&self, request: RpcRequest) -> RpcResponse {
        let mut rpc_response = RpcResponse::new();

        let _list_units_request: ListUnitsRequest = match Message::parse_from_bytes(&request.data) {
            Ok(request) => {
                rpc_response.method = EnumOrUnknown::from(RpcMethod::ListUnits);
                request
            },
            Err(error) => {
                warn!("Failed to parse list units request: {}", error);
                rpc_response.method = EnumOrUnknown::from(RpcMethod::ListUnits);
                rpc_response.status = false;
                rpc_response.error = format!("Failed to parse list units request: {}", error);
                return rpc_response;
            },
        };

        debug!("Received list units request");

        let mut list_units_response = ListUnitsResponse::new();

        match self.unit_manager.lock() {
            Ok(unit_manager) => {
                list_units_response.units = self.convert_units_to_proto(&unit_manager.get_units())
            },
            Err(error) => {
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
        match unit.lock() {
            Ok(mut unit) => {
                let mut proto_unit = tsm_unitman_rpc::Unit::new();

                proto_unit.name = unit.get_name().clone();
                proto_unit.executable = unit.get_executable().clone();
                proto_unit.arguments = unit.get_arguments().clone();
                proto_unit.restart_policy = EnumOrUnknown::from_i32(unit.get_restart_policy().clone() as i32);
                proto_unit.uid = unit.get_uid() as i32;
                proto_unit.gid = unit.get_gid() as i32;
                proto_unit.enabled = unit.is_enabled();
                proto_unit.probe_state = EnumOrUnknown::from_i32(unit.get_probe_state().clone() as i32);

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