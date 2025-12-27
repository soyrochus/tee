use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::app::commands::shared;
use crate::app::error::{AppErrorKind, AppErrorSource};
use crate::data::task_repo;
use crate::domain::{
    policy,
    principal::Principal,
    types::{TaskDescription, TaskPriority, TaskTitle},
};

#[derive(Debug)]
pub struct CreateTaskCommand {
    pub title_raw: String,
    pub description_raw: String,
    pub due_at: Option<DateTime<Utc>>,
    pub priority_raw: Option<i16>,
}

#[derive(Debug, thiserror::Error)]
pub enum CreateTaskError {
    #[error("not authorized")]
    NotAuthorized,
    #[error("invalid title")]
    InvalidTitle(#[from] crate::domain::types::TaskTitleError),
    #[error("invalid description")]
    InvalidDescription(#[from] crate::domain::types::TaskDescriptionError),
    #[error("invalid priority")]
    InvalidPriority(#[from] crate::domain::types::TaskPriorityError),
    #[error("database error")]
    Db(#[from] sqlx::Error),
}

impl AppErrorSource for CreateTaskError {
    fn error_kind(&self) -> AppErrorKind {
        match self {
            CreateTaskError::NotAuthorized => AppErrorKind::Forbidden,
            CreateTaskError::Db(_) => AppErrorKind::Db,
            _ => AppErrorKind::BadRequest,
        }
    }

    fn user_message(&self) -> String {
        match self {
            CreateTaskError::NotAuthorized => "You are not allowed to create tasks.".to_string(),
            CreateTaskError::InvalidTitle(e) => e.to_string(),
            CreateTaskError::InvalidDescription(e) => e.to_string(),
            CreateTaskError::InvalidPriority(e) => e.to_string(),
            CreateTaskError::Db(_) => "Unexpected database error.".to_string(),
        }
    }

    fn into_db_error(self) -> Option<sqlx::Error> {
        match self {
            CreateTaskError::Db(err) => Some(err),
            _ => None,
        }
    }
}

pub async fn handle(
    pool: &PgPool,
    principal: &Principal,
    cmd: CreateTaskCommand,
) -> Result<Uuid, CreateTaskError> {
    shared::ensure_authorized(
        policy::can_create_task(principal),
        CreateTaskError::NotAuthorized,
    )?;

    let title = TaskTitle::parse(&cmd.title_raw)?;
    let description = TaskDescription::parse(&cmd.description_raw)?;
    let priority = TaskPriority::parse(cmd.priority_raw.unwrap_or(3))?;
    let mut tx = pool.begin().await?;
    let id = task_repo::insert_task(
        &mut *tx,
        title.as_str(),
        description.as_str(),
        cmd.due_at,
        priority.as_i16(),
    )
    .await?;
    tx.commit().await?;
    tracing::info!(task_id = %id, "task created");
    Ok(id)
}
