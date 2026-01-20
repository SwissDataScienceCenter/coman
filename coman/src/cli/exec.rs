use std::{thread, time::Duration};

use base64::prelude::*;
use color_eyre::Result;
use iroh::{
    Endpoint, SecretKey,
    protocol::{ProtocolHandler, Router},
};
use iroh_ssh::IrohSsh;
use pid1::Pid1Settings;
use rust_supervisor::{ChildType, Supervisor, SupervisorConfig};
use tokio::{net::TcpStream, task::JoinSet};

const SECRET_KEY_ENV: &str = "COMAN_IROH_SECRET";
const PORT_FORWARD_ENV: &str = "COMAN_FORWARDED_PORTS";

fn get_secret_key() -> Option<Vec<u8>> {
    if let Ok(secret) = std::env::var(SECRET_KEY_ENV) {
        let secret_key = BASE64_STANDARD.decode(secret).unwrap();
        Some(secret_key)
    } else {
        None
    }
}

#[tokio::main]
async fn run_ssh() -> Result<()> {
    let mut builder = IrohSsh::builder().accept_incoming(true).accept_port(15263);
    if let Some(secret_key) = get_secret_key() {
        let secret_key: &[u8; 32] = secret_key[0..32].try_into().unwrap();
        builder = builder.secret_key(secret_key);
    }
    let server = builder.build().await.expect("couldn't create iroh server");
    println!("{}@{}", whoami::username(), server.node_id());
    tokio::signal::ctrl_c().await?;
    Ok(())
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

                        let a_to_b = async move { tokio::io::copy(&mut local_read, &mut iroh_send).await };
                        let b_to_a = async move { tokio::io::copy(&mut iroh_recv, &mut local_write).await };

                        tokio::select! {
                            result = a_to_b => {
                                println!("{port}->Iroh stream ended: {result:?}");
                            },
                            result = b_to_a => {
                                println!("Iroh->{port} stream ended: {result:?}");
                            },
                        };
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
    if let Ok(forwarded_ports) = std::env::var(PORT_FORWARD_ENV) {
        println!("setting up port forwarding...");
        let mut join_set = JoinSet::new();
        for port in forwarded_ports.split(',') {
            let alpn: Vec<u8> = format!("/coman/{port}").into_bytes();
            let endpoint = Endpoint::builder()
                .secret_key(secret_key.clone())
                .alpns(vec![alpn.clone()])
                .bind()
                .await?;

            let port = port.to_owned();
            join_set.spawn(async move {
                let handler = PortForwardHandler {
                    port: port.parse::<u16>().expect("couldn't parse port"),
                };
                Router::builder(endpoint.clone()).accept(&alpn, handler).spawn();
            });
        }
        while let Some(res) = join_set.join_next().await {
            println!("Task joined: {res:?}");
        }
    }

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
    supervisor.add_process("iroh-ssh", ChildType::Permanent, || {
        thread::spawn(|| {
            let _ = run_ssh();
        })
    });
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
