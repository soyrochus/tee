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
pub struct StartTaskCommand {
    pub id: Uuid,
    pub expected_row_version: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum StartTaskError {
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

impl AppErrorSource for StartTaskError {
    fn error_kind(&self) -> AppErrorKind {
        match self {
            StartTaskError::NotAuthorized => AppErrorKind::Forbidden,
            StartTaskError::NotFound => AppErrorKind::NotFound,
            StartTaskError::TaskDeleted | StartTaskError::ConcurrencyConflict => {
                AppErrorKind::Conflict
            }
            StartTaskError::Db(_) => AppErrorKind::Db,
            _ => AppErrorKind::BadRequest,
        }
    }

    fn user_message(&self) -> String {
        match self {
            StartTaskError::NotAuthorized => "You are not allowed to start tasks.".to_string(),
            StartTaskError::NotFound => "Task not found.".to_string(),
            StartTaskError::TaskDeleted => "Task was deleted.".to_string(),
            StartTaskError::InvalidTransition => {
                "Task cannot be started from its current state.".to_string()
            }
            StartTaskError::InvalidStatus => "Task is in an invalid state.".to_string(),
            StartTaskError::ConcurrencyConflict => {
                "Task was modified by someone else. Please refresh.".to_string()
            }
            StartTaskError::Db(_) => "Unexpected database error.".to_string(),
        }
    }

    fn into_db_error(self) -> Option<sqlx::Error> {
        match self {
            StartTaskError::Db(err) => Some(err),
            _ => None,
        }
    }
}

pub async fn handle(
    pool: &PgPool,
    principal: &Principal,
    cmd: StartTaskCommand,
) -> Result<(), StartTaskError> {
    shared::ensure_authorized(
        policy::can_start_task(principal),
        StartTaskError::NotAuthorized,
    )?;

    let mut tx = pool.begin().await?;
    let row = shared::fetch_task_required(&mut *tx, cmd.id, StartTaskError::NotFound).await?;
    let current = shared::parse_status(&row.status, StartTaskError::InvalidStatus)?;
    if !can_transition_task(row.is_deleted, current, TaskStatus::InProgress) {
        if row.is_deleted {
            return Err(StartTaskError::TaskDeleted);
        }
        tracing::warn!(
            task_id = %cmd.id,
            from = %current,
            to = %TaskStatus::InProgress,
            "invalid task transition attempt"
        );
        return Err(StartTaskError::InvalidTransition);
    }
    let updated = task_repo::update_task_status(
        &mut *tx,
        cmd.id,
        TaskStatus::InProgress,
        cmd.expected_row_version,
    )
    .await?;
    if updated == 0 {
        return match shared::classify_update_conflict(&mut *tx, cmd.id).await? {
            UpdateConflict::NotFound => Err(StartTaskError::NotFound),
            UpdateConflict::Deleted => Err(StartTaskError::TaskDeleted),
            UpdateConflict::Conflict => Err(StartTaskError::ConcurrencyConflict),
        };
    }
    tx.commit().await?;
    tracing::info!(task_id = %cmd.id, from = %current, to = %TaskStatus::InProgress, "task started");
    Ok(())
}
