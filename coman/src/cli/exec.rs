use std::{thread, time::Duration};

use base64::prelude::*;
use color_eyre::Result;
use iroh::{
    Endpoint, SecretKey,
    endpoint::ConnectionError,
    protocol::{ProtocolHandler, Router},
};
use pid1::Pid1Settings;
use rust_supervisor::{ChildType, Supervisor, SupervisorConfig};
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::cli::rpc::{COMAN_RPC_ALPN, RpcHandler};

const SECRET_KEY_ENV: &str = "COMAN_IROH_SECRET";
const PORT_FORWARD_ENV: &str = "COMAN_FORWARDED_PORTS";
const SSH_PORT: u16 = 15263;

fn get_secret_key() -> Option<Vec<u8>> {
    if let Ok(secret) = std::env::var(SECRET_KEY_ENV) {
        let secret_key = BASE64_STANDARD.decode(secret).unwrap();
        Some(secret_key)
    } else {
        None
    }
}

#[derive(Debug)]
struct PortForwardHandler {
    port: u16,
}

impl ProtocolHandler for PortForwardHandler {
    async fn accept(&self, connection: iroh::endpoint::Connection) -> Result<(), iroh::protocol::AcceptError> {
        let endpoint_id = connection.remote_id();
        let port = self.port;

        match connection.accept_bi().await {
            Ok((mut iroh_send, mut iroh_recv)) => {
                println!("Accepted bidirectional stream from {endpoint_id}");

                match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
                    Ok(mut output_stream) => {
                        println!("Connected to local server on port {}", port);

                        let (mut local_read, mut local_write) = output_stream.split();

                        let a_to_b = async move {
                            let res = tokio::io::copy(&mut local_read, &mut iroh_send).await;
                            if res.is_ok() {
                                iroh_send.flush().await.expect("couldn't flush stream");
                                iroh_send.finish().expect("couldn't finish stream");
                                iroh_send.stopped().await.expect("stream not properly stopped");
                            }
                            res
                        };
                        let b_to_a = async move { tokio::io::copy(&mut iroh_recv, &mut local_write).await };

                        tokio::select! {
                            result = a_to_b => {
                                println!("{port}->Iroh stream ended: {result:?}");
                            },
                            result = b_to_a => {
                                println!("Iroh->{port} stream ended: {result:?}");
                            },
                        };
                        // wait for client to close connection so we don't close prematurely
                        let res = tokio::time::timeout(Duration::from_secs(3), async move {
                            let closed = connection.closed().await;
                            if !matches!(closed, ConnectionError::ApplicationClosed(_)) {
                                println!("endpoint disconnected witn an error: {closed:#}");
                            } else {
                                println!("connection closed");
                            }
                        })
                        .await;
                        if res.is_err() {
                            println!("endpoint did not disconnect within 3 seconds");
                        }
                    }
                    Err(e) => {
                        println!("Failed to connect to local server {port}: {e}");
                    }
                }
            }
            Err(e) => {
                println!("Failed to accept bidirectional stream {port}: {e}");
            }
        }

        Ok(())
    }
}
#[tokio::main]
async fn port_forward() -> Result<()> {
    let Some(secret_key) = get_secret_key() else {
        return Ok(());
    };
    let secret_key: &[u8; 32] = secret_key[0..32].try_into().unwrap();
    let secret_key = SecretKey::from_bytes(secret_key);
    let mut forwarded_ports = vec!["ssh".to_owned()];
    if let Ok(env_ports) = std::env::var(PORT_FORWARD_ENV) {
        forwarded_ports.extend(env_ports.split(',').map(|p| p.to_owned()).collect::<Vec<String>>());
    }
    let endpoint = Endpoint::builder().secret_key(secret_key.clone()).bind().await?;
    let id = endpoint.id();
    println!("endpoint: {id}");

    println!("setting up port forwarding...");
    let mut builder = Router::builder(endpoint.clone());
    for port in forwarded_ports {
        let (port, alpn) = if port == "ssh" {
            (SSH_PORT, "/iroh/ssh".to_string())
        } else {
            (
                port.parse::<u16>().expect("couldn't parse port"),
                format!("/coman/{port}"),
            )
        };

        let handler = PortForwardHandler { port };
        builder = builder.accept(alpn.clone().into_bytes(), handler);
        println!("set up port forwarding for port {port} ({alpn})");
    }

    // add rpc server
    let rpc_handler = RpcHandler;
    builder = builder.accept(COMAN_RPC_ALPN, rpc_handler);
    let _router = builder.spawn();
    println!("port forwarding started");

    let _ = tokio::signal::ctrl_c().await;
    println!("port forwarding stopped");
    Ok(())
}

/// Runs a wrapped command in a container-safe way and potentially runs background processes like iroh-ssh
pub(crate) async fn cli_exec_command(command: Vec<String>) -> Result<()> {
    // Pid1 takes care of proper terminating of processes and signal handling when running in a container
    Pid1Settings::new()
        .enable_log(true)
        .timeout(Duration::from_secs(2))
        .launch()
        .expect("Launch failed");

    let mut supervisor = Supervisor::new(SupervisorConfig::default());
    supervisor.add_process("port-forward", ChildType::Permanent, || {
        thread::spawn(|| {
            let _ = port_forward();
        })
    });
    supervisor.add_process("main-process", ChildType::Temporary, move || {
        let command = command.clone();
        thread::spawn(move || {
            let mut child = std::process::Command::new(command[0].clone())
                .args(&command[1..])
                .spawn()
                .expect("Failed to start compute job");
            child.wait().expect("Failed to wait on compute job");
        })
    });

    let supervisor = supervisor.start_monitoring();
    loop {
        thread::sleep(Duration::from_secs(1));

        if let Some(rust_supervisor::ProcessState::Failed | rust_supervisor::ProcessState::Stopped) =
            supervisor.get_process_state("main-process")
        {
            break;
        }
    }
    Ok(())
}
