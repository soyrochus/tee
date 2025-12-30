use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Planned,
    InProgress,
    Completed,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Planned => "PLANNED",
            TaskStatus::InProgress => "IN_PROGRESS",
            TaskStatus::Completed => "COMPLETED",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TaskStatusParseError {
    #[error("invalid status")]
    Invalid,
}

impl TaskStatus {
    pub fn parse(raw: &str) -> Result<Self, TaskStatusParseError> {
        match raw {
            "PLANNED" => Ok(TaskStatus::Planned),
            "IN_PROGRESS" => Ok(TaskStatus::InProgress),
            "COMPLETED" => Ok(TaskStatus::Completed),
            _ => Err(TaskStatusParseError::Invalid),
        }
    }
}

pub fn can_transition(from: TaskStatus, to: TaskStatus) -> bool {
    matches!(
        (from, to),
        (TaskStatus::Planned, TaskStatus::InProgress)
            | (TaskStatus::InProgress, TaskStatus::Completed)
    )
}

pub fn can_transition_task(is_deleted: bool, from: TaskStatus, to: TaskStatus) -> bool {
    if is_deleted {
        return false;
    }
    can_transition(from, to)
}

pub fn is_mutable(is_deleted: bool) -> bool {
    !is_deleted
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_at: Option<DateTime<Utc>>,
    pub priority: i16,
    pub row_version: i64,
    #[allow(dead_code)]
    pub is_deleted: bool,
    #[allow(dead_code)]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::{can_transition, can_transition_task, is_mutable, TaskStatus};

    #[test]
    fn transition_matrix_matches_spec() {
        assert!(can_transition(TaskStatus::Planned, TaskStatus::InProgress));
        assert!(!can_transition(TaskStatus::Planned, TaskStatus::Completed));
        assert!(can_transition(
            TaskStatus::InProgress,
            TaskStatus::Completed
        ));
        assert!(!can_transition(TaskStatus::InProgress, TaskStatus::Planned));
        assert!(!can_transition(TaskStatus::Completed, TaskStatus::Planned));
        assert!(!can_transition(
            TaskStatus::Completed,
            TaskStatus::InProgress
        ));
        assert!(!can_transition(
            TaskStatus::Completed,
            TaskStatus::Completed
        ));
    }

    #[test]
    fn deleted_tasks_are_immutable() {
        assert!(!is_mutable(true));
        assert!(is_mutable(false));
    }

    #[test]
    fn deleted_tasks_reject_transitions() {
        assert!(!can_transition_task(
            true,
            TaskStatus::Planned,
            TaskStatus::InProgress
        ));
    }
}
