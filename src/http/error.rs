use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Serialize, Serializer};

fn serialize_status_code<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u16(status.as_u16())
}

pub trait ErrorName {
    fn name(&self) -> &'static str;
}

impl<T> ErrorName for T
where
    for<'a> &'a T: Into<&'static str>,
{
    fn name(&self) -> &'static str {
        self.into()
    }
}

pub trait HttpError: std::error::Error + ErrorName {
    fn status(&self) -> StatusCode;
}

#[derive(Debug, derive_more::Error, derive_more::From, derive_more::Display)]
pub struct JsonRejection(axum::extract::rejection::JsonRejection);

impl IntoResponse for JsonRejection {
    fn into_response(self) -> Response {
        let error = ApiErrorResponse {
            name: "JsonRejection",
            status: self.0.status(),
            message: self.0.body_text(),
        };

        (error.status, axum::Json(error)).into_response()
    }
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub struct ApiError<E: HttpError>(E);

#[derive(Serialize)]
struct ApiErrorResponse {
    name: &'static str,
    #[serde(serialize_with = "serialize_status_code")]
    status: StatusCode,
    message: String,
}

impl<E: HttpError> IntoResponse for ApiError<E> {
    fn into_response(self) -> Response {
        let status = self.0.status();

        let response = ApiErrorResponse {
            name: self.0.name(),
            message: self.0.to_string(),
            status,
        };

        (status, axum::Json(response)).into_response()
    }
}
