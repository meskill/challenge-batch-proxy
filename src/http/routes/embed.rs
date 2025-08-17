use crate::embedding::{
    EmbedError, EmbedUpstreamRequest, EmbedUpstreamResponse, TruncationDirection,
    UpstreamErrorResponse,
};
use crate::http::error::ApiError;
use crate::http::extractors::Json;
use crate::state::AppState;
use crate::types::embedding::Embedding;
use axum::extract::State;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct EmbedRequest {
    pub input: String,
    pub normalize: Option<bool>,
    pub prompt_name: Option<String>,
    pub truncate: Option<bool>,
    pub truncation_direction: Option<TruncationDirection>,
}

#[derive(Debug, Serialize)]
pub struct EmbedResponse(Embedding);

pub async fn embed(
    State(state): State<AppState>,
    Json(payload): Json<EmbedRequest>,
) -> Result<Json<EmbedResponse>, ApiError<EmbedError>> {
    match handler(state, payload).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => Err(ApiError::from(error)),
    }
}

async fn handler(state: AppState, payload: EmbedRequest) -> Result<EmbedResponse, EmbedError> {
    let upstream_request = EmbedUpstreamRequest {
        inputs: payload.input,
        normalize: payload.normalize.unwrap_or(true),
        prompt_name: payload.prompt_name,
        truncate: payload.truncate.unwrap_or(false),
        truncation_direction: payload
            .truncation_direction
            .unwrap_or(TruncationDirection::Right),
    };

    let response = state.embedding.upstream.embed(&upstream_request).await?;

    // Handle upstream errors
    if !response.status().is_success() {
        let status = response.status();
        tracing::error!(%status, "Text-embedding service returned error status");

        // Try to parse error response
        match response.json::<UpstreamErrorResponse>().await {
            Ok(error_response) => {
                return Err(EmbedError::from_upstream_response(status, error_response));
            }
            Err(_) => {
                // If we can't parse the error response, create a generic error
                return Err(EmbedError::Unknown {
                    status_code: status.as_u16(),
                    message: format!("Upstream service error: {}", status),
                });
            }
        }
    }

    // Parse successful response
    let mut response: EmbedUpstreamResponse = response.json().await?;

    // Check if response contains embeddings
    if response.0.is_empty() {
        return Err(EmbedError::EmptyResponse);
    }

    // TODO: use the first embedding only for now
    Ok(EmbedResponse(response.0.remove(0)))
}
