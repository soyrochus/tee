use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::data::task_repo;
use crate::domain::task::TaskStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateConflict {
    NotFound,
    Deleted,
    Conflict,
}

pub fn ensure_authorized<Err>(allowed: bool, err: Err) -> Result<(), Err> {
    if allowed {
        Ok(())
    } else {
        Err(err)
    }
}

pub async fn fetch_task_required<'a, E, ErrType>(
    executor: E,
    id: Uuid,
    not_found: ErrType,
) -> Result<task_repo::TaskRow, ErrType>
where
    E: Executor<'a, Database = Postgres>,
    ErrType: From<sqlx::Error>,
{
    task_repo::fetch_task(executor, id).await?.ok_or(not_found)
}

pub fn parse_status<ErrType>(raw: &str, err: ErrType) -> Result<TaskStatus, ErrType> {
    TaskStatus::parse(raw).map_err(|_| err)
}

pub async fn classify_update_conflict<'a, E>(
    executor: E,
    id: Uuid,
) -> Result<UpdateConflict, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let current_row = task_repo::fetch_task(executor, id).await?;
    Ok(match current_row {
        None => UpdateConflict::NotFound,
        Some(row) if row.is_deleted => UpdateConflict::Deleted,
        Some(_) => UpdateConflict::Conflict,
    })
}
