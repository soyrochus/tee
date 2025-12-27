use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::domain::task::TaskStatus;

#[derive(Debug)]
pub struct TaskRow {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_at: Option<DateTime<Utc>>,
    pub priority: i16,
    pub row_version: i64,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

pub async fn insert_task<'a, E>(
    executor: E,
    title: &str,
    description: &str,
    due_at: Option<DateTime<Utc>>,
    priority: i16,
) -> Result<Uuid, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let rec = sqlx::query!(
        r#"
        INSERT INTO tasks (title, description, status, created_at, updated_at, due_at, priority, row_version, is_deleted)
        VALUES ($1, $2, $3, now(), now(), $4, $5, 1, FALSE)
        RETURNING id
        "#,
        title,
        description,
        TaskStatus::Planned.as_str(),
        due_at,
        priority
    )
    .fetch_one(executor)
    .await?;

    Ok(rec.id)
}

pub async fn fetch_task<'a, E>(executor: E, id: Uuid) -> Result<Option<TaskRow>, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let row = sqlx::query_as!(
        TaskRow,
        r#"
        SELECT id, title, description, status, created_at, updated_at, due_at, priority, row_version, is_deleted, deleted_at
        FROM tasks
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(executor)
    .await?;

    Ok(row)
}

pub async fn fetch_task_active<'a, E>(executor: E, id: Uuid) -> Result<Option<TaskRow>, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let row = sqlx::query_as!(
        TaskRow,
        r#"
        SELECT id, title, description, status, created_at, updated_at, due_at, priority, row_version, is_deleted, deleted_at
        FROM tasks
        WHERE id = $1 AND is_deleted = FALSE
        "#,
        id
    )
    .fetch_optional(executor)
    .await?;

    Ok(row)
}

#[derive(Debug)]
pub struct ListTasksParams<'a> {
    pub status: Option<&'a str>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub search: Option<&'a str>,
    pub priority: Option<i16>,
    pub sort: Option<&'a str>,
    pub limit: i64,
}

pub async fn list_tasks<'a, E>(
    executor: E,
    params: ListTasksParams<'a>,
) -> Result<Vec<TaskRow>, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let rows = sqlx::query_as!(
        TaskRow,
        r#"
        SELECT id, title, description, status, created_at, updated_at, due_at, priority, row_version, is_deleted, deleted_at
        FROM tasks
        WHERE is_deleted = FALSE
          AND ($1::text IS NULL OR status = $1)
          AND ($2::timestamptz IS NULL OR created_at >= $2)
          AND ($3::timestamptz IS NULL OR created_at <= $3)
          AND ($4::text IS NULL OR title ILIKE '%' || $4 || '%' OR description ILIKE '%' || $4 || '%')
          AND ($5::smallint IS NULL OR priority = $5)
        ORDER BY
          CASE WHEN $6 = 'due_at' THEN due_at END ASC NULLS LAST,
          CASE WHEN $6 = 'priority' THEN priority END ASC,
          CASE WHEN $6 = 'updated_at' THEN updated_at END DESC,
          updated_at DESC
        LIMIT $7
        "#,
        params.status,
        params.created_after,
        params.created_before,
        params.search,
        params.priority,
        params.sort,
        params.limit
    )
    .fetch_all(executor)
    .await?;

    Ok(rows)
}

pub async fn update_task_details<'a, E>(
    executor: E,
    id: Uuid,
    title: &str,
    description: &str,
    due_at: Option<DateTime<Utc>>,
    priority: i16,
    expected_row_version: i64,
) -> Result<u64, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let result = sqlx::query!(
        r#"
        UPDATE tasks
        SET title = $2,
            description = $3,
            due_at = $4,
            priority = $5,
            updated_at = now(),
            row_version = row_version + 1
        WHERE id = $1
          AND row_version = $6
          AND is_deleted = FALSE
        "#,
        id,
        title,
        description,
        due_at,
        priority,
        expected_row_version
    )
    .execute(executor)
    .await?;

    Ok(result.rows_affected())
}

pub async fn update_task_status<'a, E>(
    executor: E,
    id: Uuid,
    status: TaskStatus,
    expected_row_version: i64,
) -> Result<u64, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let result = sqlx::query!(
        r#"
        UPDATE tasks
        SET status = $2,
            updated_at = now(),
            row_version = row_version + 1
        WHERE id = $1
          AND row_version = $3
          AND is_deleted = FALSE
        "#,
        id,
        status.as_str(),
        expected_row_version
    )
    .execute(executor)
    .await?;

    Ok(result.rows_affected())
}

pub async fn soft_delete_task<'a, E>(
    executor: E,
    id: Uuid,
    expected_row_version: i64,
) -> Result<u64, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let result = sqlx::query!(
        r#"
        UPDATE tasks
        SET is_deleted = TRUE,
            deleted_at = now(),
            updated_at = now(),
            row_version = row_version + 1
        WHERE id = $1
          AND row_version = $2
          AND is_deleted = FALSE
        "#,
        id,
        expected_row_version
    )
    .execute(executor)
    .await?;

    Ok(result.rows_affected())
}
