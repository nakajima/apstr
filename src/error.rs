use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub struct AppError(anyhow::Error);

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error = ?self.0, "request failed");
        (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
    }
}
