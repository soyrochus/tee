use crate::data::task_repo::TaskRow;
use crate::domain::task::{Task, TaskStatus, TaskStatusParseError};

pub fn task_from_row(row: TaskRow) -> Result<Task, TaskStatusParseError> {
    let status = TaskStatus::parse(&row.status)?;
    Ok(Task {
        id: row.id,
        title: row.title,
        description: row.description,
        status,
        created_at: row.created_at,
        updated_at: row.updated_at,
        due_at: row.due_at,
        priority: row.priority,
        row_version: row.row_version,
        is_deleted: row.is_deleted,
        deleted_at: row.deleted_at,
    })
}
