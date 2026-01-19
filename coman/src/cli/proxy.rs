use color_eyre::{Result, eyre::eyre};

use crate::{
    config::get_data_dir,
    cscs::{api_client::types::JobStatus, handlers::cscs_job_details},
};

/// Thin wrapper around iroh proxy
pub(crate) async fn cli_proxy_command(system: String, job_id: i64) -> Result<()> {
    let data_dir = get_data_dir();
    let job_info = cscs_job_details(job_id, Some(system.clone()), None).await?;
    if job_info.is_none() {
        return Err(eyre!("remote job does not exist!"));
    } else if let Some(job_info) = job_info
        && job_info.status != JobStatus::Running
    {
        return Err(eyre!("remote job is not in running state, connection not available"));
    }
    let endpoint_id = std::fs::read_to_string(data_dir.join(format!("{}_{}.endpoint", system, job_id)))?;
    println!("{}", endpoint_id);
    iroh_ssh::api::proxy_mode(iroh_ssh::ProxyArgs { node_id: endpoint_id })
        .await
        .map_err(|e| eyre!("couldn't proxy ssh connection: {:?}", e))
}
