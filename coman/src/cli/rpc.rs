use bytesize::ByteSize;
use futures::StreamExt;
use iroh::protocol::ProtocolHandler;
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
    pub gpu: Option<Vec<(u64, u64)>>,
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
        let gpu_usage = if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(vec![
                "--query-gpu=memory.total,memory.used",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            let output = String::from_utf8_lossy(&output.stdout);
            let usage = output
                .lines()
                .map(|l| l.split_once(",").unwrap())
                .map(|(total, used)| {
                    (
                        ByteSize::mib(total.trim().parse::<u64>().unwrap()).as_u64(),
                        ByteSize::mib(used.trim().parse::<u64>().unwrap()).as_u64(),
                    )
                })
                .collect();
            Some(usage)
        } else {
            println!("Failed to execute nvidia-smi, maybe it's not installed");
            None
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
