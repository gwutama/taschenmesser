use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::AsyncReadExt;
use std::net::ToSocketAddrs;
use tokio::net::TcpListener;
use tokio_util::compat::TokioAsyncReadCompatExt;
use log::warn;

use crate::tsm_unitman_capnp::unit_man;
use crate::unit::ManagerRef;


pub struct RpcServer {
    unit_manager: ManagerRef,
}


impl unit_man::Server for RpcServer {
    fn get_units(
        &mut self,
        params: unit_man::GetUnitsParams,
        mut results: unit_man::GetUnitsResults,
    ) -> Promise<(), capnp::Error> {
        results.get().init_units(0);
        Promise::ok(())
    }
}


impl RpcServer {
    pub fn new(unit_manager: ManagerRef) -> RpcServer {
        RpcServer {
            unit_manager,
        }
    }

    pub async fn run_threaded(unit_manager: ManagerRef) -> Result<(), Box<dyn std::error::Error>> {
        tokio::task::LocalSet::new()
            .run_until(async move {
                let listener = TcpListener::bind("127.0.0.1:8080").await?;
                let rpc_server = RpcServer::new(unit_manager.clone());
                let client: unit_man::Client = capnp_rpc::new_client(rpc_server);

                loop {
                    let (stream, _) = listener.accept().await?;
                    stream.set_nodelay(true)?;
                    let (reader, writer) = TokioAsyncReadCompatExt::compat(stream).split();
                    let network = twoparty::VatNetwork::new(
                        reader,
                        writer,
                        rpc_twoparty_capnp::Side::Server,
                        Default::default(),
                    );

                    let rpc_system =
                        RpcSystem::new(Box::new(network), Some(client.clone().client));

                    tokio::task::spawn_local(rpc_system);
                }
            })
            .await
    }
}