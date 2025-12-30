#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppErrorKind {
    Forbidden,
    NotFound,
    Conflict,
    BadRequest,
    Internal,
    Db,
}

pub trait AppErrorSource {
    fn error_kind(&self) -> AppErrorKind;
    fn user_message(&self) -> String;
    fn into_db_error(self) -> Option<sqlx::Error>
    where
        Self: Sized,
    {
        None
    }
}
