use std::thread;
use std::time::Duration;
use log::{debug};
use protobuf::Message;

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
use tsm_unitman_rpc::{RpcRequest, RpcResponse, AckRequest, AckResponse};


pub struct RpcServer {}


impl RpcServer {
    pub fn run_threaded() -> thread::JoinHandle<()> {
        debug!("Spawning thread");

        let thread_handle = thread::spawn(move || {
            Self::run();
        });

        return thread_handle;
    }

    pub fn run() {
        let context = zmq::Context::new();
        let responder = context.socket(zmq::REP).unwrap();

        assert!(responder.bind("ipc:///tmp/tsm-unitman.sock").is_ok());

        loop {
            // https://github.com/pronebird/node-rust-zeromq/blob/master/server/src/main.rs
            let bytes = responder.recv_bytes(0).unwrap();
            let request: RpcRequest = Message::parse_from_bytes(&bytes).unwrap();
            debug!("Received request: {:?}", request);

            // handle request
            let response = Self::handle_request(&request);
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