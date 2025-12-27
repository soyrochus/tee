use sqlx::PgPool;
use uuid::Uuid;

use crate::app::commands::shared;
use crate::app::commands::shared::UpdateConflict;
use crate::app::error::{AppErrorKind, AppErrorSource};
use crate::data::task_repo;
use crate::domain::{
    policy,
    principal::Principal,
    task::{can_transition_task, TaskStatus},
};

#[derive(Debug)]
pub struct CompleteTaskCommand {
    pub id: Uuid,
    pub expected_row_version: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum CompleteTaskError {
    #[error("not authorized")]
    NotAuthorized,
    #[error("task not found")]
    NotFound,
    #[error("task deleted")]
    TaskDeleted,
    #[error("invalid status transition")]
    InvalidTransition,
    #[error("corrupt status data")]
    InvalidStatus,
    #[error("concurrency conflict")]
    ConcurrencyConflict,
    #[error("database error")]
    Db(#[from] sqlx::Error),
}

impl AppErrorSource for CompleteTaskError {
    fn error_kind(&self) -> AppErrorKind {
        match self {
            CompleteTaskError::NotAuthorized => AppErrorKind::Forbidden,
            CompleteTaskError::NotFound => AppErrorKind::NotFound,
            CompleteTaskError::TaskDeleted | CompleteTaskError::ConcurrencyConflict => {
                AppErrorKind::Conflict
            }
            CompleteTaskError::Db(_) => AppErrorKind::Db,
            _ => AppErrorKind::BadRequest,
        }
    }

    fn user_message(&self) -> String {
        match self {
            CompleteTaskError::NotAuthorized => {
                "You are not allowed to complete tasks.".to_string()
            }
            CompleteTaskError::NotFound => "Task not found.".to_string(),
            CompleteTaskError::TaskDeleted => "Task was deleted.".to_string(),
            CompleteTaskError::InvalidTransition => {
                "Task cannot be completed from its current state.".to_string()
            }
            CompleteTaskError::InvalidStatus => "Task is in an invalid state.".to_string(),
            CompleteTaskError::ConcurrencyConflict => {
                "Task was modified by someone else. Please refresh.".to_string()
            }
            CompleteTaskError::Db(_) => "Unexpected database error.".to_string(),
        }
    }

    fn into_db_error(self) -> Option<sqlx::Error> {
        match self {
            CompleteTaskError::Db(err) => Some(err),
            _ => None,
        }
    }
}

pub async fn handle(
    pool: &PgPool,
    principal: &Principal,
    cmd: CompleteTaskCommand,
) -> Result<(), CompleteTaskError> {
    shared::ensure_authorized(
        policy::can_complete_task(principal),
        CompleteTaskError::NotAuthorized,
    )?;

    let mut tx = pool.begin().await?;
    let row = shared::fetch_task_required(&mut *tx, cmd.id, CompleteTaskError::NotFound).await?;
    let current = shared::parse_status(&row.status, CompleteTaskError::InvalidStatus)?;
    if !can_transition_task(row.is_deleted, current, TaskStatus::Completed) {
        if row.is_deleted {
            return Err(CompleteTaskError::TaskDeleted);
        }
        tracing::warn!(
            task_id = %cmd.id,
            from = %current,
            to = %TaskStatus::Completed,
            "invalid task transition attempt"
        );
        return Err(CompleteTaskError::InvalidTransition);
    }
    let updated = task_repo::update_task_status(
        &mut *tx,
        cmd.id,
        TaskStatus::Completed,
        cmd.expected_row_version,
    )
    .await?;
    if updated == 0 {
        return match shared::classify_update_conflict(&mut *tx, cmd.id).await? {
            UpdateConflict::NotFound => Err(CompleteTaskError::NotFound),
            UpdateConflict::Deleted => Err(CompleteTaskError::TaskDeleted),
            UpdateConflict::Conflict => Err(CompleteTaskError::ConcurrencyConflict),
        };
    }
    tx.commit().await?;
    tracing::info!(task_id = %cmd.id, from = %current, to = %TaskStatus::Completed, "task completed");
    Ok(())
}
