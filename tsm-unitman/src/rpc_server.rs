use std::thread;
use std::time::Duration;
use log::{debug};
use protobuf::Message;

use unit::ManagerRef;

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
use tsm_unitman_rpc::{RpcRequest, RpcResponse, AckRequest, AckResponse};
use crate::unit;


pub struct RpcServer {
    unit_manager: ManagerRef,
}


impl RpcServer {
    pub fn new(unit_manager: ManagerRef) -> Self {
        Self {
            unit_manager,
        }
    }

    pub fn run_threaded(&self) -> thread::JoinHandle<()> {
        debug!("Spawning thread");

        let thread_handle = thread::spawn(move || {
            Self::run();
        });

        return thread_handle;
    }

    fn run() {
        let context = zmq::Context::new();
        let responder = context.socket(zmq::REP).unwrap();

        assert!(responder.bind("ipc:///tmp/tsm-unitman.sock").is_ok());

        loop {
            // https://github.com/pronebird/node-rust-zeromq/blob/master/server/src/main.rs
            let request: Option<RpcRequest> = match responder.recv_bytes(0) {
                Ok(bytes) => Some(Message::parse_from_bytes(&bytes).unwrap()),
                Err(e) => None,
            };

            // handle request
            let response: Option<RpcResponse> = match request {
                Some(request) => {
                    debug!("Received request: {:?}", request);
                    Some(Self::handle_request(&request))
                },
                None => None,
            };

            match response {
                Some(response) => {
                    debug!("Sending response: {:?}", response);
                    let message = response.write_to_bytes().unwrap();
                    responder.send(&message, 0).unwrap();
                },
                None => (),
            }
        }
    }

    fn handle_request(request: &RpcRequest) -> RpcResponse {
        let method_name: &str = request.method.as_str();
        let data = &request.data;

        let mut rpc_response = RpcResponse::new();
        rpc_response.method = method_name.to_string();

        let result = match method_name {
            ".rpc.Service.ack" => {
                let mut ack = AckResponse::new();
                ack.message = "pong".to_string();
                Ok(ack)
            },
            _ => Err("Unknown method"),
        };

        match result {
            Ok(response) => {
                rpc_response.status = true;
                rpc_response.data = response.write_to_bytes().unwrap();
            },
            Err(e) => {
                rpc_response.status = false;
                rpc_response.error = e.to_string();
            }
        }

        return rpc_response;
    }
}