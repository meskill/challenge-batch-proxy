use std::future::Ready;

use crate::state::AppState;
use serde::Serialize;

use axum::extract::State;

use axum::Json;

use axum::http::StatusCode;

use axum::response::IntoResponse;

#[derive(Serialize)]
pub(crate) struct HealthResponse<'a> {
    pub(crate) status: &'a str,
}

pub(crate) async fn health() -> Json<HealthResponse<'static>> {
    let body = HealthResponse { status: "ok" };
    Json(body)
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "status")]
pub(crate) enum ReadyResponse<'details> {
    Ready,
    NotReady { details: &'details str },
}

impl IntoResponse for ReadyResponse<'_> {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            ReadyResponse::Ready => StatusCode::OK,
            ReadyResponse::NotReady { .. } => StatusCode::SERVICE_UNAVAILABLE,
        };
        let body = Json(self);
        (status, body).into_response()
    }
}


pub(crate) async fn ready(
    State(state): State<AppState>,
) -> ReadyResponse<'static> {
    ReadyResponse::NotReady { details: "not ready yet" }
}
