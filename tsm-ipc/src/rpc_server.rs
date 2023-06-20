use std::thread;
use std::sync::{Arc, Mutex};
use log::{debug, warn};
use protobuf::Message;

use crate::tsm_common_rpc;


pub trait RpcRequestHandler: Send {
    fn handle_request(&self, request: tsm_common_rpc::RpcRequest) -> tsm_common_rpc::RpcResponse;
}


pub struct RpcServer {
    bind_address: String,
    request_handler: Arc<Mutex<dyn RpcRequestHandler>>,
}


impl RpcServer {
    pub fn new(bind_address: String, request_handler: Arc<Mutex<dyn RpcRequestHandler>>) -> Self {
        Self {
            bind_address,
            request_handler,
        }
    }

    pub fn run_threaded(self) -> thread::JoinHandle<()> {
        debug!("Spawning thread");
        thread::spawn(move || {
            self.run();
        })
    }

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
            let request: Option<tsm_common_rpc::RpcRequest> = match responder.recv_bytes(0) {
                Ok(bytes) => Some(Message::parse_from_bytes(&bytes).unwrap()),
                Err(_) => None,
            };

            // handle request
            let response: Option<tsm_common_rpc::RpcResponse> = match request {
                Some(request) => {
                    debug!("Received request: {:?}", request);

                    match self.request_handler.lock() {
                        Ok(request_handler) => {
                            Some(request_handler.handle_request(request))
                        },
                        Err(error) => {
                            warn!("Failed to lock request handler: {}", error);
                            None
                        },
                    }
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
}