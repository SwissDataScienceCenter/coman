use std::path::PathBuf;

use eyre::{Result, eyre};

use crate::{
    client::FirecrestClient,
    types::{GetFileTailResponse, PostMakeDirRequest, PostMkdirResponse, PutFileChmodRequest, PutFileChmodResponse},
};

pub async fn post_filesystem_ops_mkdir(
    client: &FirecrestClient,
    system_name: &str,
    path: PathBuf,
) -> Result<PostMkdirResponse> {
    let body = PostMakeDirRequest {
        parent: Some(true),
        source_path: Some(
            path.into_os_string()
                .into_string()
                .map_err(|_| eyre!("couldn't convert path"))?,
        ),
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
