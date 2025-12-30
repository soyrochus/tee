use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::app::error::{AppErrorKind, AppErrorSource};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("authentication required")]
    AuthenticationRequired,
    #[error("csrf violation")]
    CsrfViolation,
    #[error("not authorized")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("template error")]
    Template(#[from] askama::Error),
    #[error("database error")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    Command(String),
    #[error("internal error: {0}")]
    Internal(String),
}

fn map_app_error<E: AppErrorSource>(err: E) -> AppError {
    match err.error_kind() {
        AppErrorKind::Forbidden => AppError::Forbidden,
        AppErrorKind::NotFound => AppError::NotFound,
        AppErrorKind::Conflict => AppError::Conflict(err.user_message()),
        AppErrorKind::BadRequest => AppError::Command(err.user_message()),
        AppErrorKind::Internal => AppError::Internal(err.user_message()),
        AppErrorKind::Db => {
            let db = err
                .into_db_error()
                .expect("db errors must provide sqlx::Error");
            AppError::Db(db)
        }
    }
}

impl<T> From<T> for AppError
where
    T: AppErrorSource,
{
    fn from(err: T) -> Self {
        map_app_error(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::AuthenticationRequired => {
                (StatusCode::UNAUTHORIZED, "authentication required").into_response()
            }
            AppError::CsrfViolation => (StatusCode::FORBIDDEN, "csrf violation").into_response(),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden").into_response(),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found").into_response(),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg).into_response(),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            AppError::Template(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
            }
            AppError::Db(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response()
            }
            AppError::Command(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
        }
    }
}
