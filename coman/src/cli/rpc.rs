use futures::StreamExt;
use iroh::protocol::ProtocolHandler;
use tarpc::{
    serde_transport as transport, server, server::Channel, tokio_serde::formats::Bincode,
    tokio_util::codec::LengthDelimitedCodec,
};
use tokio_duplex::Duplex;

use crate::cli::app::COMAN_VERSION;

#[tarpc::service]
pub trait ComanRPC {
    async fn version() -> String;
}
#[derive(Debug, Clone)]
struct RpcServer;

impl ComanRPC for RpcServer {
    async fn version(self, _: tarpc::context::Context) -> String {
        COMAN_VERSION.to_string()
    }
}

#[derive(Debug, Default)]
pub struct RpcHandler;

impl ProtocolHandler for RpcHandler {
    async fn accept(&self, connection: iroh::endpoint::Connection) -> Result<(), iroh::protocol::AcceptError> {
        let endpoint_id = connection.remote_id();
        match connection.accept_bi().await {
            Ok((iroh_send, iroh_recv)) => {
                println!("Accepted bidirectional stream from {endpoint_id}");
                let codec_builder = LengthDelimitedCodec::builder();
                let combined = Duplex::new(iroh_recv, iroh_send);
                let framed = codec_builder.new_framed(combined);

                let transport = transport::new(framed, Bincode::default());
                let server = server::BaseChannel::with_defaults(transport);
                tokio::spawn(server.execute(RpcServer.serve()).for_each(spawn));
            }
            Err(e) => {
                println!("Failed to accept bidirectional stream to rpc: {e}");
            }
        }

        Ok(())
    }
}

async fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
}
