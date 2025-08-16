//! Uses `axum::extract::FromRequest` to wrap another extractor and customize the
//! rejection
use axum::{extract::FromRequest, response::IntoResponse};
use serde::Serialize;

use crate::http::error::JsonRejection;

// create an extractor that internally uses `axum::Json` but has a custom rejection
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(JsonRejection))]
pub struct Json<T>(pub T);

// We implement `IntoResponse` for our extractor so it can be used as a response
impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}
