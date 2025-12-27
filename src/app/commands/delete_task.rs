use sqlx::PgPool;
use uuid::Uuid;

use crate::app::commands::shared;
use crate::app::commands::shared::UpdateConflict;
use crate::app::error::{AppErrorKind, AppErrorSource};
use crate::data::task_repo;
use crate::domain::{policy, principal::Principal};

#[derive(Debug)]
pub struct DeleteTaskCommand {
    pub id: Uuid,
    pub expected_row_version: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum DeleteTaskError {
    #[error("not authorized")]
    NotAuthorized,
    #[error("task not found")]
    NotFound,
    #[error("concurrency conflict")]
    ConcurrencyConflict,
    #[error("database error")]
    Db(#[from] sqlx::Error),
}

impl AppErrorSource for DeleteTaskError {
    fn error_kind(&self) -> AppErrorKind {
        match self {
            DeleteTaskError::NotAuthorized => AppErrorKind::Forbidden,
            DeleteTaskError::NotFound => AppErrorKind::NotFound,
            DeleteTaskError::ConcurrencyConflict => AppErrorKind::Conflict,
            DeleteTaskError::Db(_) => AppErrorKind::Db,
        }
    }

    fn user_message(&self) -> String {
        match self {
            DeleteTaskError::NotAuthorized => "You are not allowed to delete tasks.".to_string(),
            DeleteTaskError::NotFound => "Task not found.".to_string(),
            DeleteTaskError::ConcurrencyConflict => {
                "Task was modified by someone else. Please refresh.".to_string()
            }
            DeleteTaskError::Db(_) => "Unexpected database error.".to_string(),
        }
    }

    fn into_db_error(self) -> Option<sqlx::Error> {
        match self {
            DeleteTaskError::Db(err) => Some(err),
            _ => None,
        }
    }
}

pub async fn handle(
    pool: &PgPool,
    principal: &Principal,
    cmd: DeleteTaskCommand,
) -> Result<(), DeleteTaskError> {
    shared::ensure_authorized(
        policy::can_delete_task(principal),
        DeleteTaskError::NotAuthorized,
    )?;

    let mut tx = pool.begin().await?;
    let row = shared::fetch_task_required(&mut *tx, cmd.id, DeleteTaskError::NotFound).await?;
    if row.is_deleted {
        tx.commit().await?;
        return Ok(());
    }

    let updated = task_repo::soft_delete_task(&mut *tx, cmd.id, cmd.expected_row_version).await?;
    if updated == 0 {
        return match shared::classify_update_conflict(&mut *tx, cmd.id).await? {
            UpdateConflict::NotFound => Err(DeleteTaskError::NotFound),
            UpdateConflict::Deleted => Ok(()),
            UpdateConflict::Conflict => Err(DeleteTaskError::ConcurrencyConflict),
        };
    }
    tx.commit().await?;
    tracing::info!(task_id = %cmd.id, "task deleted");
    Ok(())
}
