use crate::embedding::{
    EmbedError, EmbedUpstreamRequest, EmbedUpstreamResponse, UpstreamErrorResponse,
};
use crate::state::AppState;
use crate::types::embedding::Embedding;
use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct EmbedRequest {
    pub input: String,
    // TODO: add other parameters
}

#[derive(Debug, Serialize)]
pub struct EmbedResponse(Embedding);

pub async fn embed(
    State(state): State<AppState>,
    Json(payload): Json<EmbedRequest>,
) -> Result<Json<EmbedResponse>, EmbedError> {
    let upstream_request = EmbedUpstreamRequest {
        inputs: payload.input,
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
    Ok(Json(EmbedResponse(response.0.remove(0))))
}
