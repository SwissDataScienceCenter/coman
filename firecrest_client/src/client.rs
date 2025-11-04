use eyre::{Result, WrapErr};
use std::{fmt, pin::Pin};

use crate::error::FirecrestError;

pub struct FirecrestClient {
    base_path: reqwest::Url,
    user_agent: Option<String>,
    reqwest_client: reqwest::Client,
    token: Option<String>,
}

impl fmt::Debug for FirecrestClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FirecrestClient")
            .field("base_path", &self.base_path)
            .field("user_agent", &self.user_agent)
            .finish()
    }
}

impl Default for FirecrestClient {
    fn default() -> Self {
        Self {
            base_path: reqwest::Url::parse("https://api.cscs.ch/hpc/firecrest/v2").unwrap(),
            user_agent: None,
            reqwest_client: reqwest::Client::new(),
            token: None,
        }
    }
}

impl FirecrestClient {
    pub fn base_path(mut self, base_path: String) -> Result<Self> {
        self.base_path = reqwest::Url::parse(&base_path)?;
        Ok(self)
    }

    pub fn user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }
    pub fn token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    async fn request(
        &self,
        path: &str,
        method: reqwest::Method,
        body: Option<String>,
        params: Option<Vec<(&str, &str)>>,
    ) -> Result<String> {
        let mut url = self.base_path.join(path)?;
        if let Some(params) = params {
            url.query_pairs_mut().extend_pairs(params);
        }
        let mut request_builder = self.reqwest_client.request(method, url);
        if let Some(user_agent) = self.user_agent.clone() {
            request_builder = request_builder.header(reqwest::header::USER_AGENT, user_agent);
        }
        if let Some(ref token) = self.token {
            request_builder = request_builder.bearer_auth(token);
        }
        if let Some(body) = body {
            request_builder = request_builder.body(body);
        }
        let req = request_builder.build()?;
        let url = req.url().clone();
        let resp = self.reqwest_client.execute(req).await?;
        let status = resp.status();

        if !status.is_client_error() && !status.is_server_error() {
            let content = resp.text().await?;
            Ok(content)
        } else {
            let content = resp.text().await?;
            Err(FirecrestError::ResponseError {
                status,
                content: content.clone(),
            })
            .wrap_err(format!("Request failed for {}: \n{}", url, content))
        }
    }
    pub async fn get(&self, path: &str, params: Option<Vec<(&str, &str)>>) -> Result<String> {
        self.request(path, reqwest::Method::GET, None, params).await
    }
    pub async fn delete(&self, path: &str, params: Option<Vec<(&str, &str)>>) -> Result<String> {
        self.request(path, reqwest::Method::DELETE, None, params)
            .await
    }
    pub async fn post(
        &self,
        path: &str,
        body: String,
        params: Option<Vec<(&str, &str)>>,
    ) -> Result<String> {
        self.request(path, reqwest::Method::POST, Some(body), params)
            .await
    }
}
