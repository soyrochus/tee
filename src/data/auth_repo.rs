use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub password_hash: String,
    pub is_active: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct SessionWithUserRow {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub user_email: String,
    pub user_display_name: String,
    pub user_is_active: bool,
}

pub async fn fetch_user_by_email<'a, E>(
    executor: E,
    email: &str,
) -> Result<Option<UserRow>, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let row = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, password_hash, is_active
        FROM users
        WHERE lower(email) = lower($1)
        "#,
    )
    .bind(email)
    .fetch_optional(executor)
    .await?;

    Ok(row)
}

pub async fn insert_session<'a, E>(
    executor: E,
    user_id: Uuid,
    session_token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<Uuid, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO sessions (user_id, session_token_hash, expires_at, last_seen_at)
        VALUES ($1, $2, $3, now())
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(session_token_hash)
    .bind(expires_at)
    .fetch_one(executor)
    .await?;

    Ok(id)
}

pub async fn fetch_session_with_user_by_hash<'a, E>(
    executor: E,
    session_token_hash: &str,
) -> Result<Option<SessionWithUserRow>, sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    let row = sqlx::query_as::<_, SessionWithUserRow>(
        r#"
        SELECT
          s.id AS session_id,
          s.user_id,
          s.expires_at,
          s.last_seen_at,
          s.revoked_at,
          u.email AS user_email,
          u.display_name AS user_display_name,
          u.is_active AS user_is_active
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.session_token_hash = $1
        "#,
    )
    .bind(session_token_hash)
    .fetch_optional(executor)
    .await?;

    Ok(row)
}

pub async fn update_session_last_seen<'a, E>(
    executor: E,
    session_id: Uuid,
) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    sqlx::query(
        r#"
        UPDATE sessions
        SET last_seen_at = now()
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn revoke_session<'a, E>(executor: E, session_id: Uuid) -> Result<(), sqlx::Error>
where
    E: Executor<'a, Database = Postgres>,
{
    sqlx::query(
        r#"
        UPDATE sessions
        SET revoked_at = now()
        WHERE id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(session_id)
    .execute(executor)
    .await?;
    Ok(())
}
