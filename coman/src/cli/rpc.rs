use futures::StreamExt;
use iroh::protocol::ProtocolHandler;
use nvml_wrapper::Nvml;
use serde::{Deserialize, Serialize};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, get_current_pid};
use tarpc::{
    serde_transport as transport, server, server::Channel, tokio_serde::formats::Bincode,
    tokio_util::codec::LengthDelimitedCodec,
};
use tokio_duplex::Duplex;

use crate::cli::app::COMAN_VERSION;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu: f32,
    pub rss: u64,
    pub vss: u64,
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
        tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu());
        let Ok(pid) = get_current_pid() else {
            return ResourceUsage::default();
        };
        let Some(process) = sys.process(pid) else {
            return ResourceUsage::default();
        };
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
            cpu: process.cpu_usage() / sys.cpus().len() as f32,
            rss: process.memory(),
            vss: process.virtual_memory(),
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
