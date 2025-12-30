use sqlx::PgPool;
use uuid::Uuid;

use crate::app::error::{AppErrorKind, AppErrorSource};
use crate::app::queries::task_mapper;
use crate::data::task_repo;
use crate::domain::{policy, principal::Principal, task::Task};

#[derive(Debug, thiserror::Error)]
pub enum GetTaskError {
    #[error("not authorized")]
    NotAuthorized,
    #[error("task not found")]
    NotFound,
    #[error("invalid status data")]
    InvalidStatus,
    #[error("database error")]
    Db(#[from] sqlx::Error),
}

impl AppErrorSource for GetTaskError {
    fn error_kind(&self) -> AppErrorKind {
        match self {
            GetTaskError::NotAuthorized => AppErrorKind::Forbidden,
            GetTaskError::NotFound => AppErrorKind::NotFound,
            GetTaskError::InvalidStatus => AppErrorKind::Internal,
            GetTaskError::Db(_) => AppErrorKind::Db,
        }
    }

    fn user_message(&self) -> String {
        match self {
            GetTaskError::NotAuthorized => "not authorized".to_string(),
            GetTaskError::NotFound => "task not found".to_string(),
            GetTaskError::InvalidStatus => "invalid status data".to_string(),
            GetTaskError::Db(_) => "database error".to_string(),
        }
    }

    fn into_db_error(self) -> Option<sqlx::Error> {
        match self {
            GetTaskError::Db(err) => Some(err),
            _ => None,
        }
    }
}

pub async fn handle(pool: &PgPool, principal: &Principal, id: Uuid) -> Result<Task, GetTaskError> {
    if !policy::can_view_task(principal) {
        return Err(GetTaskError::NotAuthorized);
    }
    let row = task_repo::fetch_task_active(pool, id)
        .await?
        .ok_or(GetTaskError::NotFound)?;
    task_mapper::task_from_row(row).map_err(|_| GetTaskError::InvalidStatus)
}
