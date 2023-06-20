use protobuf::Message;

use crate::tsm_common_rpc;


pub struct RpcClient{
    zmq_socket: zmq::Socket
}


impl RpcClient {
    pub fn new(bind_address: String) -> Result<RpcClient, String> {
        let zmq_context = zmq::Context::new();
        let zmq_socket = match zmq_context.socket(zmq::REQ) {
            Ok(socket) => socket,
            Err(error) => return Err(format!("Failed to create ZMQ socket: {}", error))
        };

        match zmq_socket.connect(&bind_address) {
            Ok(_) => {},
            Err(error) => return Err(format!("Failed to connect to ZMQ socket: {}", error))
        };

        Ok(RpcClient {
            zmq_socket
        })
    }

    pub fn send(&self, request: tsm_common_rpc::RpcRequest) -> Result<tsm_common_rpc::RpcResponse, String> {
        let message = match request.write_to_bytes() {
            Ok(bytes) => bytes,
            Err(error) => return Err(format!("Failed to serialize request: {}", error))
        };

        match self.zmq_socket.send(&message, 0) {
            Ok(_) => {},
            Err(error) => return Err(format!("Failed to send request: {}", error))
        };

        let bytes = match self.zmq_socket.recv_bytes(0) {
            Ok(bytes) => bytes,
            Err(error) => return Err(format!("Failed to receive response: {}", error))
        };

        return match bytes.len() {
            0 => Err(String::from("Received empty response")),
            _ => {
                match Message::parse_from_bytes(&bytes) {
                    Ok(message) => Ok(message),
                    Err(error) => Err(format!("Failed to parse response: {}", error))
                }
            }
        };
    }
}