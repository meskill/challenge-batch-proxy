pub mod embed;
pub mod status;

use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use tower_http::trace::TraceLayer;

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(status::health))
        .route("/ready", get(status::ready))
        .route("/embed", post(embed::embed))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
