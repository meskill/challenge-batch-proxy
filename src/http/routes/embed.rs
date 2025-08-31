use crate::embedding::{EmbedError, EmbedRequest, EmbedResponse};
use crate::http::error::ApiError;
use crate::http::extractors::Json;
use crate::state::AppState;
use axum::extract::State;

pub async fn embed(
    State(state): State<AppState>,
    Json(payload): Json<EmbedRequest>,
) -> Result<Json<EmbedResponse>, ApiError<EmbedError>> {
    match handler(state, payload).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => Err(ApiError::from(error)),
    }
}

#[tracing::instrument(skip_all)]
async fn handler(state: AppState, payload: EmbedRequest) -> Result<EmbedResponse, EmbedError> {
    state.embedding.embed(payload).await
}
