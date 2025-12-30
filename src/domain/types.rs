#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskTitle(String);

impl TaskTitle {
    pub fn parse(raw: &str) -> Result<Self, TaskTitleError> {
        let s = raw.trim();
        if s.is_empty() {
            return Err(TaskTitleError::Empty);
        }
        if s.chars().count() > 200 {
            return Err(TaskTitleError::TooLong);
        }
        Ok(Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskDescription(String);

impl TaskDescription {
    pub fn parse(raw: &str) -> Result<Self, TaskDescriptionError> {
        let s = raw.trim();
        if s.is_empty() {
            return Err(TaskDescriptionError::Empty);
        }
        if s.chars().count() > 4000 {
            return Err(TaskDescriptionError::TooLong);
        }
        Ok(Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskPriority(i16);

impl TaskPriority {
    pub fn parse(raw: i16) -> Result<Self, TaskPriorityError> {
        if (1..=5).contains(&raw) {
            Ok(Self(raw))
        } else {
            Err(TaskPriorityError::OutOfRange)
        }
    }

    pub fn as_i16(self) -> i16 {
        self.0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TaskTitleError {
    #[error("title must not be empty")]
    Empty,
    #[error("title must be at most 200 characters")]
    TooLong,
}

#[derive(Debug, thiserror::Error)]
pub enum TaskDescriptionError {
    #[error("description must not be empty")]
    Empty,
    #[error("description must be at most 4000 characters")]
    TooLong,
}

#[derive(Debug, thiserror::Error)]
pub enum TaskPriorityError {
    #[error("priority must be between 1 and 5")]
    OutOfRange,
}

#[cfg(test)]
mod tests {
    use super::{TaskDescription, TaskPriority, TaskTitle};

    #[test]
    fn title_validation_rejects_empty() {
        assert!(TaskTitle::parse("  ").is_err());
    }

    #[test]
    fn title_validation_rejects_too_long() {
        let too_long = "a".repeat(201);
        assert!(TaskTitle::parse(&too_long).is_err());
    }

    #[test]
    fn title_validation_accepts_max_length() {
        let ok = "a".repeat(200);
        assert!(TaskTitle::parse(&ok).is_ok());
    }

    #[test]
    fn description_validation_rejects_empty() {
        assert!(TaskDescription::parse("  ").is_err());
    }

    #[test]
    fn description_validation_rejects_too_long() {
        let too_long = "a".repeat(4001);
        assert!(TaskDescription::parse(&too_long).is_err());
    }

    #[test]
    fn description_validation_accepts_max_length() {
        let ok = "a".repeat(4000);
        assert!(TaskDescription::parse(&ok).is_ok());
    }

    #[test]
    fn priority_validation_rejects_out_of_range() {
        assert!(TaskPriority::parse(0).is_err());
        assert!(TaskPriority::parse(6).is_err());
    }

    #[test]
    fn priority_validation_accepts_bounds() {
        assert!(TaskPriority::parse(1).is_ok());
        assert!(TaskPriority::parse(5).is_ok());
    }
}
