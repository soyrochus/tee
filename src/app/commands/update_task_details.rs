use sqlx::PgPool;
use uuid::Uuid;

use crate::app::commands::shared;
use crate::app::commands::shared::UpdateConflict;
use crate::app::error::{AppErrorKind, AppErrorSource};
use crate::data::task_repo;
use chrono::{DateTime, Utc};

use crate::domain::{
    policy,
    principal::Principal,
    task::{is_mutable, TaskStatus},
    types::{TaskDescription, TaskPriority, TaskTitle},
};

#[derive(Debug)]
pub struct UpdateTaskDetailsCommand {
    pub id: Uuid,
    pub title_raw: String,
    pub description_raw: String,
    pub due_at: Option<DateTime<Utc>>,
    pub priority_raw: i16,
    pub expected_row_version: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateTaskDetailsError {
    #[error("not authorized")]
    NotAuthorized,
    #[error("task not found")]
    NotFound,
    #[error("task deleted")]
    TaskDeleted,
    #[error("task is completed")]
    Completed,
    #[error("invalid title")]
    InvalidTitle(#[from] crate::domain::types::TaskTitleError),
    #[error("invalid description")]
    InvalidDescription(#[from] crate::domain::types::TaskDescriptionError),
    #[error("invalid priority")]
    InvalidPriority(#[from] crate::domain::types::TaskPriorityError),
    #[error("corrupt status data")]
    InvalidStatus,
    #[error("concurrency conflict")]
    ConcurrencyConflict,
    #[error("database error")]
    Db(#[from] sqlx::Error),
}

impl AppErrorSource for UpdateTaskDetailsError {
    fn error_kind(&self) -> AppErrorKind {
        match self {
            UpdateTaskDetailsError::NotAuthorized => AppErrorKind::Forbidden,
            UpdateTaskDetailsError::NotFound => AppErrorKind::NotFound,
            UpdateTaskDetailsError::TaskDeleted | UpdateTaskDetailsError::ConcurrencyConflict => {
                AppErrorKind::Conflict
            }
            UpdateTaskDetailsError::Db(_) => AppErrorKind::Db,
            _ => AppErrorKind::BadRequest,
        }
    }

    fn user_message(&self) -> String {
        match self {
            UpdateTaskDetailsError::NotAuthorized => {
                "You are not allowed to update tasks.".to_string()
            }
            UpdateTaskDetailsError::NotFound => "Task not found.".to_string(),
            UpdateTaskDetailsError::TaskDeleted => "Task was deleted.".to_string(),
            UpdateTaskDetailsError::Completed => "Completed tasks are read-only.".to_string(),
            UpdateTaskDetailsError::InvalidTitle(e) => e.to_string(),
            UpdateTaskDetailsError::InvalidDescription(e) => e.to_string(),
            UpdateTaskDetailsError::InvalidPriority(e) => e.to_string(),
            UpdateTaskDetailsError::InvalidStatus => "Task is in an invalid state.".to_string(),
            UpdateTaskDetailsError::ConcurrencyConflict => {
                "Task was modified by someone else. Please refresh.".to_string()
            }
            UpdateTaskDetailsError::Db(_) => "Unexpected database error.".to_string(),
        }
    }

    fn into_db_error(self) -> Option<sqlx::Error> {
        match self {
            UpdateTaskDetailsError::Db(err) => Some(err),
            _ => None,
        }
    }
}

pub async fn handle(
    pool: &PgPool,
    principal: &Principal,
    cmd: UpdateTaskDetailsCommand,
) -> Result<(), UpdateTaskDetailsError> {
    shared::ensure_authorized(
        policy::can_update_task_details(principal),
        UpdateTaskDetailsError::NotAuthorized,
    )?;

    let mut tx = pool.begin().await?;
    let row =
        shared::fetch_task_required(&mut *tx, cmd.id, UpdateTaskDetailsError::NotFound).await?;
    if row.is_deleted {
        return Err(UpdateTaskDetailsError::TaskDeleted);
    }
    let current = shared::parse_status(&row.status, UpdateTaskDetailsError::InvalidStatus)?;
    if current == TaskStatus::Completed {
        tracing::warn!(task_id = %cmd.id, "update attempt on completed task");
        return Err(UpdateTaskDetailsError::Completed);
    }
    if !is_mutable(row.is_deleted) {
        return Err(UpdateTaskDetailsError::TaskDeleted);
    }

    let title = TaskTitle::parse(&cmd.title_raw)?.as_str().to_string();
    let description = TaskDescription::parse(&cmd.description_raw)?
        .as_str()
        .to_string();
    let priority = TaskPriority::parse(cmd.priority_raw)?;

    let updated = task_repo::update_task_details(
        &mut *tx,
        cmd.id,
        &title,
        &description,
        cmd.due_at,
        priority.as_i16(),
        cmd.expected_row_version,
    )
    .await?;
    if updated == 0 {
        return match shared::classify_update_conflict(&mut *tx, cmd.id).await? {
            UpdateConflict::NotFound => Err(UpdateTaskDetailsError::NotFound),
            UpdateConflict::Deleted => Err(UpdateTaskDetailsError::TaskDeleted),
            UpdateConflict::Conflict => Err(UpdateTaskDetailsError::ConcurrencyConflict),
        };
    }
    tx.commit().await?;
    tracing::info!(task_id = %cmd.id, "task details updated");
    Ok(())
}
