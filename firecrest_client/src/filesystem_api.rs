use std::path::PathBuf;

use eyre::{Result, eyre};

use crate::{
    client::FirecrestClient,
    types::{
        DownloadFileResponse, DownloadFileResponseTransferDirectives, GetDirectoryLsResponse, GetFileStatResponse,
        GetFileTailResponse, PostFileDownloadRequest, PostFileDownloadRequestTransferDirectives, PostFileUploadRequest,
        PostMakeDirRequest, PostMkdirResponse, PutFileChmodRequest, PutFileChmodResponse, S3TransferRequest,
        S3TransferResponse, UploadFileResponse,
    },
};

pub async fn get_filesystem_ops_ls(
    client: &FirecrestClient,
    system_name: &str,
    path: PathBuf,
) -> Result<GetDirectoryLsResponse> {
    let path = path.as_os_str().to_str().ok_or(eyre!("couldn't cast path to string"))?;
    let response = client
        .get(
            format!("filesystem/{system_name}/ops/ls").as_str(),
            Some(vec![("path", path)]),
        )
        .await?;
    let model: GetDirectoryLsResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}
pub async fn get_filesystem_ops_stat(
    client: &FirecrestClient,
    system_name: &str,
    path: PathBuf,
) -> Result<GetFileStatResponse> {
    let path = path.as_os_str().to_str().ok_or(eyre!("couldn't cast path to string"))?;
    let response = client
        .get(
            format!("filesystem/{system_name}/ops/stat").as_str(),
            Some(vec![("path", path)]),
        )
        .await?;
    let model: GetFileStatResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}

pub async fn post_filesystem_ops_mkdir(
    client: &FirecrestClient,
    system_name: &str,
    path: PathBuf,
) -> Result<PostMkdirResponse> {
    let path = path
        .into_os_string()
        .into_string()
        .map_err(|_| eyre!("couldn't convert path"))?;
    let body = PostMakeDirRequest {
        parent: Some(true),
        source_path: Some(path),
    };
    let body_json = serde_json::to_string(&body)?;
    let response = client
        .post(
            format!("filesystem/{system_name}/ops/mkdir").as_str(),
            body_json,
            None,
            None,
        )
        .await?;
    let model: PostMkdirResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}

pub async fn put_filesystem_ops_chmod(
    client: &FirecrestClient,
    system_name: &str,
    path: PathBuf,
    mode: &str,
) -> Result<PutFileChmodResponse> {
    let body = PutFileChmodRequest {
        source_path: Some(
            path.into_os_string()
                .into_string()
                .map_err(|_| eyre!("couldn't convert path"))?,
        ),
        mode: mode.to_owned(),
    };
    let body_json = serde_json::to_string(&body)?;
    let response = client
        .put(format!("filesystem/{system_name}/ops/chmod").as_str(), body_json, None)
        .await?;
    let model: PutFileChmodResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}

pub async fn get_filesystem_ops_tail(
    client: &FirecrestClient,
    system_name: &str,
    path: PathBuf,
    lines: usize,
) -> Result<GetFileTailResponse> {
    let path = path.as_os_str().to_str().ok_or(eyre!("couldn't cast path to string"))?;
    let response = client
        .get(
            format!("filesystem/{system_name}/ops/tail").as_str(),
            Some(vec![("path", path), ("lines", &lines.to_string())]),
        )
        .await?;
    let model: GetFileTailResponse = serde_json::from_str(response.as_str())?;
    Ok(model)
}

pub async fn post_filesystem_ops_upload(
    client: &FirecrestClient,
    system_name: &str,
    path: PathBuf,
    file: Vec<u8>,
) -> Result<()> {
    let folder = path
        .parent()
        .ok_or(eyre!("couldn't get parent folder"))?
        .as_os_str()
        .to_str()
        .ok_or(eyre!("couldn't cast parent folder to string"))?;
    let filename = path
        .file_name()
        .ok_or(eyre!("couldn't get file name"))?
        .to_str()
        .ok_or(eyre!("couldn't cast file name to string"))?;
    let _ = client
        .post(
            format!("filesystem/{system_name}/ops/upload").as_str(),
            "".to_owned(),
            Some(vec![("path", folder)]),
            Some(("file", (filename, file))),
        )
        .await?;
    Ok(())
}

pub async fn post_filesystem_transfer_upload(
    client: &FirecrestClient,
    system_name: &str,
    account: Option<String>,
    path: PathBuf,
    size: i64,
) -> Result<UploadFileResponse> {
    let file_path = path.as_os_str().to_str().ok_or(eyre!("couldn't cast path to string"))?;
    let body = PostFileUploadRequest {
        source_path: Some(file_path.to_owned()),
        transfer_directives: PostFileDownloadRequestTransferDirectives::S3(S3TransferRequest {
            file_size: Some(size),
            ..Default::default()
        }),
        account,
    };
    let body_json = serde_json::to_string(&body)?;
    let response = client
        .post(
            format!("filesystem/{system_name}/transfer/upload").as_str(),
            body_json,
            None,
            None,
        )
        .await?;
    let json_data: serde_json::Value = serde_json::from_str(response.as_str())?;

    let mut model: UploadFileResponse = serde_json::from_str(response.as_str())?;
    // deserializing of contained enum does not work properly as the fields are wrong, we do it here manually
    let transfer_json = json_data["transferDirectives"].clone();
    let transfer_dir = DownloadFileResponseTransferDirectives::S3(S3TransferResponse {
        complete_upload_url: transfer_json["complete_upload_url"].as_str().map(|s| s.to_owned()),
        download_url: transfer_json["download_url"].as_str().map(|s| s.to_owned()),
        max_part_size: transfer_json["max_part_size"].as_i64(),
        parts_upload_urls: transfer_json["parts_upload_urls"]
            .as_array()
            .map(|v| v.iter().flat_map(|u| u.as_str().map(|s| s.to_owned())).collect()),
        transfer_method: "s3".to_owned(),
    });
    model.transfer_directives = transfer_dir;

    Ok(model)
}

pub async fn get_filesystem_ops_download(client: &FirecrestClient, system_name: &str, path: PathBuf) -> Result<String> {
    let file_path = path.as_os_str().to_str().ok_or(eyre!("couldn't cast path to string"))?;
    let response = client
        .get(
            format!("filesystem/{system_name}/ops/download").as_str(),
            Some(vec![("path", file_path)]),
        )
        .await?;
    Ok(response.as_str().to_string())
}

pub async fn post_filesystem_transfer_download(
    client: &FirecrestClient,
    system_name: &str,
    account: Option<String>,
    path: PathBuf,
) -> Result<DownloadFileResponse> {
    let file_path = path.as_os_str().to_str().ok_or(eyre!("couldn't cast path to string"))?;
    let body = PostFileDownloadRequest {
        source_path: Some(file_path.to_owned()),
        transfer_directives: PostFileDownloadRequestTransferDirectives::S3(S3TransferRequest::default()),
        account,
    };
    let body_json = serde_json::to_string(&body)?;
    let response = client
        .post(
            format!("filesystem/{system_name}/transfer/download").as_str(),
            body_json,
            None,
            None,
        )
        .await?;
    let mut model: DownloadFileResponse = serde_json::from_str(response.as_str())?;
    let json_data: serde_json::Value = serde_json::from_str(response.as_str())?;
    let transfer_json = json_data["transferDirectives"].clone();
    let transfer_dir = DownloadFileResponseTransferDirectives::S3(S3TransferResponse {
        complete_upload_url: transfer_json["complete_upload_url"].as_str().map(|s| s.to_owned()),
        download_url: transfer_json["download_url"].as_str().map(|s| s.to_owned()),
        max_part_size: transfer_json["max_part_size"].as_i64(),
        parts_upload_urls: transfer_json["parts_upload_urls"]
            .as_array()
            .map(|v| v.iter().flat_map(|u| u.as_str().map(|s| s.to_owned())).collect()),
        transfer_method: "s3".to_owned(),
    });
    model.transfer_directives = transfer_dir;
    Ok(model)
}
