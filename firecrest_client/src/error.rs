use thiserror::Error;

#[derive(Debug, Error)]
pub enum FirecrestError {
    #[error("Invalid response (got {status:?})")]
    ResponseError {
        status: reqwest::StatusCode,
        content: String,
    },
}
