include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
use tsm_unitman_rpc::{RpcRequest, RpcResponse, AckRequest, AckResponse};
use protobuf::Message;

fn main() {
    let ctx = zmq::Context::new();

    let socket = ctx.socket(zmq::REQ).unwrap();
    socket.connect("ipc:///tmp/tsm-unitman.sock").unwrap();

    let mut request = RpcRequest::new();
    request.method = ".rpc.Service.ack".to_string();
    request.data = AckRequest::new().write_to_bytes().unwrap();

    let message = request.write_to_bytes().unwrap();

    socket.send(message, 0).unwrap();

    let response: Option<RpcRequest> = match socket.recv_bytes(0) {
        Ok(bytes) => Some(Message::parse_from_bytes(&bytes).unwrap()),
        Err(e) => None,
    };

    match response {
        Some(response) => {
            // println!("Received response: {:?}", response);
            let ack_response: AckResponse = Message::parse_from_bytes(&response.data).unwrap();
            println!("Received response: {:?}", ack_response.message);
        },
        None => (),
    }
}
