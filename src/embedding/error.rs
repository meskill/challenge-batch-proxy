use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct UpstreamErrorResponse {
    pub error: String,
    pub error_type: String,
}

#[derive(
    Debug, derive_more::Display, derive_more::Error, derive_more::From, strum::IntoStaticStr,
)]
pub enum EmbedError {
    #[from]
    #[display("HTTP request failed: {_0}")]
    HttpRequest(reqwest::Error),

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

    #[from]
    #[display("Failed to parse upstream response: {_0}")]
    ParseError(serde_json::Error),

    #[display("Empty response from upstream service")]
    EmptyResponse,
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

impl IntoResponse for EmbedError {
    fn into_response(self) -> Response {
        let status = match self {
            EmbedError::HttpRequest(_) => StatusCode::BAD_GATEWAY,
            EmbedError::BatchSize { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            EmbedError::Tokenization { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            EmbedError::Inference { .. } => StatusCode::FAILED_DEPENDENCY,
            EmbedError::Overloaded { .. } => StatusCode::TOO_MANY_REQUESTS,
            EmbedError::Unknown { .. } => StatusCode::BAD_GATEWAY,
            EmbedError::ParseError(_) => StatusCode::BAD_GATEWAY,
            EmbedError::EmptyResponse => StatusCode::BAD_GATEWAY,
        };

        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
            name: &'static str,
        }

        let response = ErrorResponse {
            message: self.to_string(),
            name: self.into(),
        };

        (status, Json(response)).into_response()
    }
}
