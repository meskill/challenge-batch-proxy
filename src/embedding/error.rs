use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::http::error::HttpError;

#[derive(Debug, Deserialize, Serialize)]
pub struct UpstreamErrorResponse {
    pub error: String,
    pub error_type: String,
}

#[derive(Debug, Clone, derive_more::Display, derive_more::Error, strum::IntoStaticStr)]
pub enum EmbedError {
    #[display("HTTP request failed: {reqwest_error}")]
    HttpRequest { reqwest_error: String },

    #[display("Batch size error: {message}")]
    BatchSize { message: String },

    #[display("Tokenization error: {message}")]
    Tokenization { message: String },

    #[display("Embedding inference failed: {message}")]
    Inference { message: String },

    #[display("Model is overloaded: {message}")]
    Overloaded { message: String },

    #[display("Unknown upstream error (status {status_code}): {message}")]
    Unknown { status_code: u16, message: String },

    #[display("Failed to parse upstream response: {serde_error}")]
    ParseError { serde_error: String },

    #[display("Empty response from upstream service")]
    EmptyResponse,
}

impl From<reqwest::Error> for EmbedError {
    fn from(value: reqwest::Error) -> Self {
        Self::HttpRequest {
            reqwest_error: value.to_string(),
        }
    }
}

impl From<serde_json::Error> for EmbedError {
    fn from(value: serde_json::Error) -> Self {
        Self::ParseError {
            serde_error: value.to_string(),
        }
    }
}

impl EmbedError {
    pub fn from_upstream_response(
        status: StatusCode,
        error_response: UpstreamErrorResponse,
    ) -> Self {
        match status.as_u16() {
            413 => Self::BatchSize {
                message: error_response.error,
            },
            422 => Self::Tokenization {
                message: error_response.error,
            },
            424 => Self::Inference {
                message: error_response.error,
            },
            429 => Self::Overloaded {
                message: error_response.error,
            },
            _ => Self::Unknown {
                status_code: status.as_u16(),
                message: error_response.error,
            },
        }
    }
}

impl HttpError for EmbedError {
    fn status(&self) -> StatusCode {
        match self {
            EmbedError::HttpRequest { .. } => StatusCode::BAD_GATEWAY,
            EmbedError::BatchSize { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            EmbedError::Tokenization { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            EmbedError::Inference { .. } => StatusCode::FAILED_DEPENDENCY,
            EmbedError::Overloaded { .. } => StatusCode::TOO_MANY_REQUESTS,
            EmbedError::Unknown { .. } => StatusCode::BAD_GATEWAY,
            EmbedError::ParseError { .. } => StatusCode::BAD_GATEWAY,
            EmbedError::EmptyResponse => StatusCode::BAD_GATEWAY,
        }
    }
}
