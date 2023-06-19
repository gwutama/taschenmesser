include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
use tsm_unitman_rpc::{RpcRequest, RpcResponse, RpcMethod,
                      AckRequest, AckResponse,
                      ListUnitsRequest, ListUnitsResponse};
use protobuf::{EnumOrUnknown, Message};

const BIND_ADDRESS: &str = "ipc:///tmp/tsm-unitman.sock";

fn main() {
    // https://github.com/erickt/rust-zmq/tree/master/examples/zguide
    let ctx = zmq::Context::new();

    let socket = ctx.socket(zmq::REQ).unwrap();
    socket.connect(BIND_ADDRESS).unwrap();

    match send_ack(&socket, String::from("ping")) {
        Some(response) => {
            let ack_response: AckResponse = Message::parse_from_bytes(&response.data).unwrap();
            println!("Ack = Received response: {:?}", ack_response.message);
        },
        None => {
            println!("Ack = No response received");
        },
    }

    match send_list_units(&socket) {
        Some(response) => {
            let list_units_response: ListUnitsResponse = Message::parse_from_bytes(&response.data).unwrap();
            println!("ListUnits = Received response: {:?}", list_units_response.units);
        },
        None => {
            println!("ListUnits = No response received");
        },
    }

    socket.disconnect(BIND_ADDRESS).unwrap();
}


fn send_ack(socket: &zmq::Socket, message: String) -> Option<RpcResponse> {
    let mut ack_request = AckRequest::new();
    ack_request.message = message;

    let mut request = RpcRequest::new();
    request.method = EnumOrUnknown::from(RpcMethod::Ack);
    request.data = ack_request.write_to_bytes().unwrap();

    return send_request(socket, request);
}


fn send_list_units(socket: &zmq::Socket) -> Option<RpcResponse> {
    let mut request = RpcRequest::new();
    request.method = EnumOrUnknown::from(RpcMethod::ListUnits);

    return send_request(socket, request);
}


fn send_request(socket: &zmq::Socket, request: RpcRequest) -> Option<RpcResponse> {
    let message = request.write_to_bytes().unwrap();
    socket.send(&message, 0).unwrap();

    let response: Option<RpcResponse> = match socket.recv_bytes(0) {
        Ok(bytes) => Some(Message::parse_from_bytes(&bytes).unwrap()),
        Err(e) => None,
    };

    return response;
}
