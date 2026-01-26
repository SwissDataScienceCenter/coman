use futures::StreamExt;
use iroh::protocol::ProtocolHandler;
use nvml_wrapper::Nvml;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use tarpc::{
    serde_transport as transport, server, server::Channel, tokio_serde::formats::Bincode,
    tokio_util::codec::LengthDelimitedCodec,
};
use tokio_duplex::Duplex;

use crate::cli::app::COMAN_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub gpu: Option<u64>,
}

#[tarpc::service]
pub trait ComanRPC {
    async fn version() -> String;
    async fn resource_usage() -> ResourceUsage;
}
#[derive(Debug, Clone)]
struct RpcServer;

impl ComanRPC for RpcServer {
    async fn version(self, _: tarpc::context::Context) -> String {
        COMAN_VERSION.to_string()
    }

    async fn resource_usage(self, _context: ::tarpc::context::Context) -> ResourceUsage {
        let mut sys = System::new_all();
        sys.refresh_all();
        let mut cpu_usage = 0.0;
        for cpu in sys.cpus() {
            cpu_usage += cpu.cpu_usage();
        }
        cpu_usage /= sys.cpus().len() as f32;
        let gpu_usage = match Nvml::init() {
            Ok(nvml) => match nvml.device_by_index(0) {
                Ok(device) => match device.memory_info() {
                    Ok(memory_info) => Some(memory_info.used),
                    Err(e) => {
                        println!("Couldn't get GPU memory info: {e:?}");
                        None
                    }
                },
                Err(e) => {
                    println!("couldn't load nvidia device 0: {e:?}");
                    None
                }
            },
            Err(e) => {
                println!("Nvidia Device Info not available: {e:?}");
                None
            }
        };
        ResourceUsage {
            cpu: cpu_usage,
            mem_used: sys.used_memory(),
            mem_total: sys.total_memory(),
            gpu: gpu_usage,
        }
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
