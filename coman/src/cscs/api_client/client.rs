use std::{collections::HashMap, path::PathBuf};

use color_eyre::eyre::{Context, Result};
use eyre::eyre;
use firecrest_client::{
    client::FirecrestClient,
    compute_api::{
        JobOptions, cancel_compute_system_job, get_compute_system_job, get_compute_system_job_metadata,
        get_compute_system_jobs, post_compute_system_job,
    },
    filesystem_api::{
        get_filesystem_ops_download, get_filesystem_ops_ls, get_filesystem_ops_stat, get_filesystem_ops_tail,
        post_filesystem_ops_mkdir, post_filesystem_ops_upload, post_filesystem_transfer_download,
        post_filesystem_transfer_upload, put_filesystem_ops_chmod,
    },
    status_api::{get_status_systems, get_status_userinfo},
    types::DownloadFileResponseTransferDirectives,
};
use reqwest::Url;

use crate::{
    config::{ComputePlatform, Config},
    cscs::api_client::types::{FileStat, Job, JobDetail, PathEntry, S3Upload, System, UserInfo},
    trace_dbg,
    util::types::DockerImageUrl,
};

#[derive(Debug, Clone, Default)]
pub struct JobStartOptions {
    pub script_file: Option<PathBuf>,
    pub image: Option<DockerImageUrl>,
    pub command: Option<Vec<String>>,
    pub stdout: Option<PathBuf>,
    pub stderr: Option<PathBuf>,
    pub container_workdir: Option<String>,
    pub env: Vec<(String, String)>,
    pub mount: Vec<(String, String)>,
}

pub struct CscsApi {
    client: FirecrestClient,
}

impl CscsApi {
    pub fn new(token: String, platform: Option<ComputePlatform>) -> Result<Self> {
        let config = Config::new()?;
        let client = FirecrestClient::default()
            .base_path(format!(
                "https://api.cscs.ch/{}/firecrest/v2/",
                platform.unwrap_or(config.cscs.current_platform)
            ))?
            .token(token);
        Ok(Self { client })
    }
    pub async fn start_job(
        &self,
        system_name: &str,
        account: Option<String>,
        name: &str,
        script_path: PathBuf,
        envvars: HashMap<String, String>,
        options: JobStartOptions,
    ) -> Result<()> {
        let workingdir = script_path.clone();
        let workingdir = workingdir.parent();
        let _result = post_compute_system_job(
            &self.client,
            system_name,
            account,
            name,
            JobOptions {
                script: None,
                script_path: Some(script_path),
                working_dir: workingdir.map(|p| p.to_path_buf()),
                envvars,
                stdout: options.stdout,
                stderr: options.stderr,
            },
        )
        .await?;

        Ok(())
    }
    pub async fn get_system(&self, system: &str) -> Result<Option<System>> {
        let systems = self.list_systems().await?;
        Ok(systems.into_iter().find(|s| s.name == system))
    }
    pub async fn list_systems(&self) -> Result<Vec<System>> {
        let result = get_status_systems(&self.client)
            .await
            .wrap_err("couldn't list CSCS systems")?;
        Ok(result.systems.into_iter().map(|s| s.into()).collect())
    }
    pub async fn list_jobs(&self, system_name: &str, all_users: Option<bool>) -> Result<Vec<Job>> {
        let result = get_compute_system_jobs(&self.client, system_name, all_users)
            .await
            .wrap_err("couldn't fetch cscs jobs")?;
        Ok(result
            .jobs
            .map(|jobs| jobs.into_iter().map(|j| j.into()).collect())
            .unwrap_or(vec![]))
    }
    pub async fn get_job(&self, system_name: &str, job_id: i64) -> Result<Option<JobDetail>> {
        let jobs = get_compute_system_job(&self.client, system_name, job_id)
            .await
            .wrap_err("couldn't fetch job info")?;
        let job = if let Some(jobs) = jobs.jobs
            && !jobs.is_empty()
        {
            jobs[0].clone()
        } else {
            return Ok(None);
        };
        let job_metadata = get_compute_system_job_metadata(&self.client, system_name, job_id)
            .await
            .wrap_err("couldn't fetch job metadata")?;
        let job_metadata = if let Some(meta) = job_metadata.jobs
            && !meta.is_empty()
        {
            meta[0].clone()
        } else {
            return Ok(None);
        };
        Ok(Some((job, job_metadata).into()))
    }

    pub async fn cancel_job(&self, system_name: &str, job_id: i64) -> Result<()> {
        cancel_compute_system_job(&self.client, system_name, job_id)
            .await
            .wrap_err("couldn't delete job")?;
        Ok(())
    }

    pub async fn mkdir(&self, system_name: &str, path: PathBuf) -> Result<()> {
        let _ = post_filesystem_ops_mkdir(&self.client, system_name, path)
            .await
            .wrap_err("couldn't create directory")?;
        Ok(())
    }
    pub async fn chmod(&self, system_name: &str, path: PathBuf, mode: &str) -> Result<()> {
        let _ = put_filesystem_ops_chmod(&self.client, system_name, path, mode)
            .await
            .wrap_err("couldn't change directory permission")?;
        Ok(())
    }
    pub async fn upload(&self, system_name: &str, path: PathBuf, file: Vec<u8>) -> Result<()> {
        post_filesystem_ops_upload(&self.client, system_name, path, file)
            .await
            .wrap_err("couldn't upload file")?;
        Ok(())
    }
    pub async fn transfer_upload(
        &self,
        system_name: &str,
        account: Option<String>,
        path: PathBuf,
        size: i64,
    ) -> Result<(i64, S3Upload)> {
        let job = post_filesystem_transfer_upload(&self.client, system_name, account, path, size)
            .await
            .wrap_err("couldn't upload file")?;
        if let DownloadFileResponseTransferDirectives::S3(directives) = job.transfer_directives {
            Ok((job.transfer_job.job_id, S3Upload::convert(directives, size as u64)?))
        } else {
            trace_dbg!(job);
            Err(eyre!("didn't get S3 transfer directive"))
        }
    }
    pub async fn download(&self, system_name: &str, path: PathBuf) -> Result<String> {
        let content = get_filesystem_ops_download(&self.client, system_name, path)
            .await
            .wrap_err("couldn't download file")?;
        Ok(content)
    }
    pub async fn transfer_download(
        &self,
        system_name: &str,
        account: Option<String>,
        path: PathBuf,
    ) -> Result<(i64, Url)> {
        let job = post_filesystem_transfer_download(&self.client, system_name, account, path)
            .await
            .wrap_err("couldn't transfer file")?;
        if let DownloadFileResponseTransferDirectives::S3(directives) = job.transfer_directives {
            let download_url = Url::parse(&directives.download_url.unwrap())?;
            Ok((job.transfer_job.job_id, download_url))
        } else {
            Err(eyre!("didn't get S3 transfer directive"))
        }
    }
    pub async fn tail(&self, system_name: &str, path: PathBuf, lines: usize) -> Result<String> {
        let result = get_filesystem_ops_tail(&self.client, system_name, path, lines)
            .await
            .wrap_err("couldn't tail file")?;
        match result.output {
            Some(output) => Ok(output.content),
            None => Ok("".to_string()),
        }
    }
    pub async fn list_path(&self, system_name: &str, path: PathBuf) -> Result<Vec<PathEntry>> {
        let result = get_filesystem_ops_ls(&self.client, system_name, path)
            .await
            .wrap_err("couldn't list path")?;
        match result.output {
            Some(entries) => Ok(entries.into_iter().map(|e| e.into()).collect()),
            None => Ok(vec![]),
        }
    }
    pub async fn stat_path(&self, system_name: &str, path: PathBuf) -> Result<Option<FileStat>> {
        let result = get_filesystem_ops_stat(&self.client, system_name, path)
            .await
            .wrap_err("couldn't stat file")?;
        Ok(result.output.map(|f| f.into()))
    }
    pub async fn get_userinfo(&self, system_name: &str) -> Result<UserInfo> {
        let result = get_status_userinfo(&self.client, system_name)
            .await
            .wrap_err("couldn't load user info")?;
        Ok(result.into())
    }
}

#[cfg(test)]
mod tests {
    use claim::*;
    use firecrest_client::types::{
        DownloadFileResponse, FirecrestFilesystemTransferModelsTransferJob, GetJobMetadataResponse, GetJobResponse,
        GetSystemsResponse, HPCCluster, JobMetadataModel, JobModel, JobStatus, PostJobSubmissionResponse,
        S3TransferResponse, UploadFileResponse,
    };
    use injectorpp::interface::injector::*;

    use super::*;

    fn get_client() -> CscsApi {
        CscsApi {
            client: FirecrestClient::default(),
        }
    }

    #[tokio::test]
    async fn test_start_job() {
        let client = get_client();
        let mut injector = InjectorPP::new();
        injector
            .when_called_async(injectorpp::async_func!(
                firecrest_client::compute_api::post_compute_system_job(
                    &client.client,
                    "",
                    None,
                    "",
                    JobOptions {
                        script: None,
                        script_path: None,
                        working_dir: None,
                        envvars: HashMap::new(),
                        stdout: None,
                        stderr: None
                    }
                ),
                Result<PostJobSubmissionResponse>
            ))
            .will_return_async(injectorpp::async_return!(
                Ok(PostJobSubmissionResponse { job_id: Some(1) }),
                Result<PostJobSubmissionResponse>
            ));
        let result = client
            .start_job(
                "test",
                None,
                "test",
                PathBuf::from("/test"),
                HashMap::new(),
                JobStartOptions::default(),
            )
            .await;
        assert_ok!(result);
    }

    #[tokio::test]
    async fn test_list_systems() {
        {
            let client = get_client();
            let mut injector = InjectorPP::new();
            injector
                .when_called_async(injectorpp::async_func!(
                    firecrest_client::status_api::get_status_systems(&client.client),
                    Result<GetSystemsResponse>
                ))
                .will_return_async(injectorpp::async_return!(
                    Ok(GetSystemsResponse {
                        systems: vec![
                            HPCCluster {
                                name: "daint".to_owned(),
                                ..Default::default()
                            },
                            HPCCluster {
                                name: "eiger".to_owned(),
                                ..Default::default()
                            }
                        ],
                        ..Default::default()
                    }),
                    Result<GetSystemsResponse>
                ));
            let result = client.list_systems().await;
            let systems = result.unwrap();
            assert_eq!(systems.len(), 2);

            let result = client.get_system("daint").await;
            let system = result.unwrap();
            assert_some!(system.clone());
            assert_eq!(system.unwrap().name, "daint");
        }
    }

    #[tokio::test]
    async fn test_list_jobs() {
        {
            let client = get_client();
            let mut injector = InjectorPP::new();
            injector
                .when_called_async(injectorpp::async_func!(
                    firecrest_client::compute_api::get_compute_system_jobs(&client.client, "", None,),
                    Result<GetJobResponse>
                ))
                .will_return_async(injectorpp::async_return!(
                    Ok(GetJobResponse {
                        jobs: Some(vec![
                            JobModel {
                                name: "Job1".to_owned(),
                                job_id: 1,
                                status: JobStatus {
                                    state: "RUNNING".to_owned(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            JobModel {
                                name: "Job2".to_owned(),
                                job_id: 2,
                                status: JobStatus {
                                    state: "FAILED".to_owned(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            JobModel {
                                name: "Job3".to_owned(),
                                job_id: 3,
                                status: JobStatus {
                                    state: "COMPLETED".to_owned(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            JobModel {
                                name: "Job4".to_owned(),
                                job_id: 4,
                                status: JobStatus {
                                    state: "PENDING".to_owned(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            JobModel {
                                name: "Job5".to_owned(),
                                job_id: 5,
                                status: JobStatus {
                                    state: "CANCELLED".to_owned(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        ])
                    }),
                    Result<GetJobResponse>
                ));
            let result = client.list_jobs("daint", None).await;
            assert_eq!(result.unwrap().len(), 5);
        }
    }

    #[tokio::test]
    async fn test_get_job() {
        {
            let client = get_client();
            let mut injector = InjectorPP::new();
            injector
                .when_called_async(injectorpp::async_func!(
                    firecrest_client::compute_api::get_compute_system_job(&client.client, "", 1,),
                    Result<GetJobResponse>
                ))
                .will_return_async(injectorpp::async_return!(
                    Ok(GetJobResponse {
                        jobs: Some(vec![JobModel {
                            name: "Job1".to_owned(),
                            job_id: 1,
                            status: JobStatus {
                                state: "RUNNING".to_owned(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }])
                    }),
                    Result<GetJobResponse>
                ));
            injector
                .when_called_async(injectorpp::async_func!(
                    firecrest_client::compute_api::get_compute_system_job_metadata(&client.client, "", 1,),
                    Result<GetJobMetadataResponse>
                ))
                .will_return_async(injectorpp::async_return!(
                    Ok(GetJobMetadataResponse {
                        jobs: Some(vec![JobMetadataModel {
                            job_id: "1".to_owned(),
                            ..Default::default()
                        }])
                    }),
                    Result<GetJobMetadataResponse>
                ));
            let result = client.get_job("test", 1).await;
            assert_ok!(result);
        }
    }

    #[tokio::test]
    async fn test_transfer_upload() {
        let client = get_client();
        let mut injector = InjectorPP::new();
        injector
            .when_called_async(injectorpp::async_func!(
                firecrest_client::filesystem_api::post_filesystem_transfer_upload(
                    &client.client,
                    "",
                    None,
                    PathBuf::from(""),
                    1,
                ),
                Result<UploadFileResponse>
            ))
            .will_return_async(injectorpp::async_return!(
                Ok(UploadFileResponse {
                    transfer_job: FirecrestFilesystemTransferModelsTransferJob {
                        job_id: 1,
                        ..Default::default()
                    },
                    transfer_directives: DownloadFileResponseTransferDirectives::S3(S3TransferResponse {
                        transfer_method: "s3".to_owned(),
                        parts_upload_urls: Some(vec!["http://test/".to_owned()]),
                        complete_upload_url: Some("http://complete".to_owned()),
                        max_part_size: Some(10000),
                        ..Default::default()
                    })
                }),
                Result<UploadFileResponse>
            ));
        let result = client.transfer_upload("test", None, PathBuf::from("/test"), 100).await;
        assert_eq!(result.unwrap().0, 1);
    }

    #[tokio::test]
    async fn test_transfer_download() {
        let client = get_client();
        let mut injector = InjectorPP::new();
        injector
            .when_called_async(injectorpp::async_func!(
                firecrest_client::filesystem_api::post_filesystem_transfer_download(
                    &client.client,
                    "",
                    None,
                    PathBuf::from(""),
                ),
                Result<DownloadFileResponse>
            ))
            .will_return_async(injectorpp::async_return!(
                Ok(DownloadFileResponse {
                    transfer_job: FirecrestFilesystemTransferModelsTransferJob {
                        job_id: 1,
                        ..Default::default()
                    },
                    transfer_directives: DownloadFileResponseTransferDirectives::S3(S3TransferResponse {
                        transfer_method: "s3".to_owned(),
                        download_url: Some("http://download".to_owned()),
                        ..Default::default()
                    })
                }),
                Result<DownloadFileResponse>
            ));
        let result = client.transfer_download("test", None, PathBuf::from("/test")).await;
        let result = result.unwrap();
        assert_eq!(result.0, 1);
        assert_eq!(result.1, Url::parse("http://download").unwrap())
    }
}
