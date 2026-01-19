use std::{thread, time::Duration};

use base64::prelude::*;
use color_eyre::Result;
use iroh_ssh::IrohSsh;
use pid1::Pid1Settings;
use rust_supervisor::{ChildType, Supervisor, SupervisorConfig};

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
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("couldn't start tokio");

            // Call the asynchronous connect method using the runtime.
            rt.block_on(async move {
                let mut builder = IrohSsh::builder().accept_incoming(true).accept_port(15263);
                if let Ok(secret) = std::env::var("COMAN_IROH_SECRET") {
                    let secret_key = BASE64_STANDARD.decode(secret).unwrap();
                    let secret_key: &[u8; 32] = secret_key[0..32].try_into().unwrap();
                    builder = builder.secret_key(secret_key);
                }

                let server = builder.build().await.expect("couldn't create iroh server");
                println!("{}@{}", whoami::username(), server.node_id());
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            });
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
