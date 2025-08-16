use crate::state::AppState;
use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(status::health))
        .route("/ready", get(status::ready))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

mod status;
