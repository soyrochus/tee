use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::app::error::{AppErrorKind, AppErrorSource};
use crate::app::queries::task_mapper;
use crate::data::task_repo;
use crate::domain::{
    policy,
    principal::Principal,
    task::{Task, TaskStatus},
};

#[derive(Debug)]
pub struct ListTasksQuery {
    pub status: Option<TaskStatus>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub search: Option<String>,
    pub priority: Option<i16>,
    pub sort: Option<String>,
    pub limit: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum ListTasksError {
    #[error("not authorized")]
    NotAuthorized,
    #[error("invalid status data")]
    InvalidStatus,
    #[error("database error")]
    Db(#[from] sqlx::Error),
}

impl AppErrorSource for ListTasksError {
    fn error_kind(&self) -> AppErrorKind {
        match self {
            ListTasksError::NotAuthorized => AppErrorKind::Forbidden,
            ListTasksError::InvalidStatus => AppErrorKind::Internal,
            ListTasksError::Db(_) => AppErrorKind::Db,
        }
    }

    fn user_message(&self) -> String {
        match self {
            ListTasksError::NotAuthorized => "not authorized".to_string(),
            ListTasksError::InvalidStatus => "invalid status data".to_string(),
            ListTasksError::Db(_) => "database error".to_string(),
        }
    }

    fn into_db_error(self) -> Option<sqlx::Error> {
        match self {
            ListTasksError::Db(err) => Some(err),
            _ => None,
        }
    }
}

pub async fn handle(
    pool: &PgPool,
    principal: &Principal,
    query: ListTasksQuery,
) -> Result<Vec<Task>, ListTasksError> {
    if !policy::can_view_tasks(principal) {
        return Err(ListTasksError::NotAuthorized);
    }
    let status_filter = query.status.map(|status| status.as_str());
    let params = task_repo::ListTasksParams {
        status: status_filter,
        created_after: query.created_after,
        created_before: query.created_before,
        search: query.search.as_deref(),
        priority: query.priority,
        sort: query.sort.as_deref(),
        limit: query.limit,
    };
    let rows = task_repo::list_tasks(pool, params).await?;
    let mut tasks = Vec::with_capacity(rows.len());
    for r in rows {
        tasks.push(task_mapper::task_from_row(r).map_err(|_| ListTasksError::InvalidStatus)?);
    }
    Ok(tasks)
}
