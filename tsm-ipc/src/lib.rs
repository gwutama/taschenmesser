include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

mod rpc_server;
pub use rpc_server::{RpcServer, RpcRequestHandler};

mod rpc_client;
pub use rpc_client::RpcClient;