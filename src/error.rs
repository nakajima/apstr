use std::backtrace::Backtrace;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub struct AppError {
    error: anyhow::Error,
    backtrace: Backtrace,
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        Self {
            error: error.into(),
            backtrace: Backtrace::force_capture(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let error_chain = self
            .error
            .chain()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        tracing::error!(
            error = %self.error,
            error_chain = ?error_chain,
            backtrace = %self.backtrace,
            "request failed"
        );

        (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
    }
}
