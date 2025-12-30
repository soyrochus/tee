use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::{header, request::Parts, HeaderMap};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::data::auth_repo;
use crate::domain::principal::Principal;
use crate::interface::error::AppError;
use crate::interface::state::{AppState, AuthSettings};

const SESSION_COOKIE_NAME: &str = "__Host-tee_session";
const LOGIN_CSRF_COOKIE_NAME: &str = "__Host-tee_login";

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub principal: Option<Principal>,
    pub csrf_token: Option<String>,
    pub session_id: Option<Uuid>,
    pub session_token: Option<String>,
}

impl AuthContext {}

#[async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let headers = &parts.headers;
        build_auth_context(&app_state, headers).await
    }
}

async fn build_auth_context(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthContext, AppError> {
    let Some(session_token) = read_cookie(headers, SESSION_COOKIE_NAME) else {
        return Ok(AuthContext {
            principal: None,
            csrf_token: None,
            session_id: None,
            session_token: None,
        });
    };

    let token_hash = hash_token(&session_token);
    let Some(row) = auth_repo::fetch_session_with_user_by_hash(&state.pool, &token_hash).await?
    else {
        return Ok(AuthContext {
            principal: None,
            csrf_token: None,
            session_id: None,
            session_token: None,
        });
    };

    let now = chrono::Utc::now();
    if row.revoked_at.is_some() {
        tracing::info!(session_id = %row.session_id, "session revoked");
        return Ok(AuthContext {
            principal: None,
            csrf_token: None,
            session_id: None,
            session_token: None,
        });
    }

    let idle_timeout = chrono::Duration::from_std(state.auth.session_idle_timeout)
        .map_err(|_| AppError::Internal("invalid idle timeout".to_string()))?;
    if row.last_seen_at + idle_timeout < now {
        auth_repo::revoke_session(&state.pool, row.session_id).await?;
        tracing::info!(session_id = %row.session_id, "session expired (idle)");
        return Ok(AuthContext {
            principal: None,
            csrf_token: None,
            session_id: None,
            session_token: None,
        });
    }

    if row.expires_at <= now {
        auth_repo::revoke_session(&state.pool, row.session_id).await?;
        tracing::info!(session_id = %row.session_id, "session expired");
        return Ok(AuthContext {
            principal: None,
            csrf_token: None,
            session_id: None,
            session_token: None,
        });
    }

    if !row.user_is_active {
        auth_repo::revoke_session(&state.pool, row.session_id).await?;
        tracing::info!(session_id = %row.session_id, "inactive user session revoked");
        return Ok(AuthContext {
            principal: None,
            csrf_token: None,
            session_id: None,
            session_token: None,
        });
    }

    auth_repo::update_session_last_seen(&state.pool, row.session_id).await?;
    let principal = Principal {
        subject: row.user_id.to_string(),
        display_name: row.user_display_name,
        email: Some(row.user_email),
        tenant_id: None,
        roles: Vec::new(),
    };
    let csrf_token = Some(derive_csrf_token(&session_token));
    Ok(AuthContext {
        principal: Some(principal),
        csrf_token,
        session_id: Some(row.session_id),
        session_token: Some(session_token),
    })
}

pub fn hash_token(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn derive_csrf_token(session_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"csrf:");
    hasher.update(session_token.as_bytes());
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}

pub fn generate_login_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn session_cookie(token: &str, settings: &AuthSettings, remember_me: bool) -> Cookie<'static> {
    let max_age = if remember_me {
        settings.remember_me_lifetime
    } else {
        settings.session_lifetime
    };
    let max_age = cookie::time::Duration::seconds(max_age.as_secs() as i64);
    Cookie::build((SESSION_COOKIE_NAME, token.to_string()))
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(max_age)
        .build()
}

pub fn clear_session_cookie() -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(0))
        .build()
}

pub fn login_csrf_cookie(token: &str) -> Cookie<'static> {
    Cookie::build((LOGIN_CSRF_COOKIE_NAME, token.to_string()))
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::minutes(30))
        .build()
}

pub fn clear_login_csrf_cookie() -> Cookie<'static> {
    Cookie::build((LOGIN_CSRF_COOKIE_NAME, ""))
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(0))
        .build()
}

pub fn validate_csrf(auth: &AuthContext, submitted: &str) -> Result<(), AppError> {
    let Some(session_token) = auth.session_token.as_deref() else {
        return Err(AppError::AuthenticationRequired);
    };
    let expected = derive_csrf_token(session_token);
    if subtle_constant_time_eq(expected.as_bytes(), submitted.as_bytes()) {
        Ok(())
    } else {
        Err(AppError::CsrfViolation)
    }
}

pub fn validate_login_csrf(headers: &HeaderMap, submitted: &str) -> Result<(), AppError> {
    let Some(cookie_value) = read_cookie(headers, LOGIN_CSRF_COOKIE_NAME) else {
        return Err(AppError::CsrfViolation);
    };
    if subtle_constant_time_eq(cookie_value.as_bytes(), submitted.as_bytes()) {
        Ok(())
    } else {
        Err(AppError::CsrfViolation)
    }
}

pub fn verify_password(password_hash: &str, password: &str) -> bool {
    let parsed = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let header = headers.get(header::COOKIE)?.to_str().ok()?;
    let mut jar = CookieJar::new();
    for value in header.split(';') {
        if let Ok(cookie) = Cookie::parse(value.trim()) {
            jar.add_original(cookie.into_owned());
        }
    }
    jar.get(name).map(|cookie| cookie.value().to_string())
}

fn subtle_constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in a.iter().zip(b.iter()) {
        diff |= left ^ right;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::Duration;

    use argon2::password_hash::{PasswordHasher, SaltString};
    use chrono::Utc;
    use sqlx::PgPool;
    use tokio::sync::OnceCell;

    use crate::interface::i18n::{Translator, DEFAULT_LOCALE, REQUIRED_LOCALES};
    use crate::interface::state::{AppState, AuthSettings, I18nState};

    static MIGRATED: OnceCell<()> = OnceCell::const_new();

    async fn test_pool() -> Option<PgPool> {
        let url = env::var("DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        ensure_migrated(&pool).await;
        Some(pool)
    }

    async fn ensure_migrated(pool: &PgPool) {
        MIGRATED
            .get_or_init(|| async {
                sqlx::migrate!("./migrations")
                    .run(pool)
                    .await
                    .expect("migrations succeed");
            })
            .await;
    }

    async fn insert_user(pool: &PgPool, email: &str, hash: &str, active: bool) -> Uuid {
        sqlx::query_scalar(
            r#"
            INSERT INTO users (email, password_hash, display_name, is_active)
            VALUES ($1, $2, 'Spec Test', $3)
            RETURNING id
            "#,
        )
        .bind(email)
        .bind(hash)
        .bind(active)
        .fetch_one(pool)
        .await
        .expect("insert user")
    }

    async fn insert_session(
        pool: &PgPool,
        user_id: Uuid,
        token_hash: &str,
        expires_at: chrono::DateTime<Utc>,
    ) -> Uuid {
        sqlx::query_scalar(
            r#"
            INSERT INTO sessions (user_id, session_token_hash, expires_at, last_seen_at)
            VALUES ($1, $2, $3, now())
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(pool)
        .await
        .expect("insert session")
    }

    async fn cleanup_auth(pool: &PgPool, user_id: Uuid, session_id: Uuid) {
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await;
    }

    fn auth_state(pool: PgPool) -> AppState {
        let translator =
            Translator::load_from_disk("locales", DEFAULT_LOCALE, REQUIRED_LOCALES).unwrap();
        let auth = AuthSettings::new(
            Duration::from_secs(12 * 60 * 60),
            Duration::from_secs(30 * 60),
            Duration::from_secs(24 * 60 * 60),
        );
        AppState {
            pool,
            auth,
            i18n: I18nState { translator },
        }
    }

    #[test]
    fn verify_password_accepts_valid_rejects_invalid() {
        let salt = SaltString::from_b64("c29tZXNhbHQxMjM0").unwrap();
        let hash = Argon2::default()
            .hash_password("password".as_bytes(), &salt)
            .unwrap()
            .to_string();
        assert!(verify_password(&hash, "password"));
        assert!(!verify_password(&hash, "not-password"));
    }

    #[test]
    fn session_cookie_has_required_attributes() {
        let settings = AuthSettings::new(
            Duration::from_secs(3600),
            Duration::from_secs(1800),
            Duration::from_secs(7200),
        );
        let cookie = session_cookie("token", &settings, false);
        assert!(cookie.name().starts_with("__Host-"));
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Lax));
    }

    #[test]
    fn csrf_validation_enforces_token_match() {
        let auth = AuthContext {
            principal: None,
            csrf_token: None,
            session_id: None,
            session_token: Some("token".to_string()),
        };
        let expected = derive_csrf_token("token");
        assert!(validate_csrf(&auth, &expected).is_ok());
        assert!(matches!(
            validate_csrf(&auth, "wrong"),
            Err(AppError::CsrfViolation)
        ));
        let no_session = AuthContext {
            principal: None,
            csrf_token: None,
            session_id: None,
            session_token: None,
        };
        assert!(matches!(
            validate_csrf(&no_session, "anything"),
            Err(AppError::AuthenticationRequired)
        ));
    }

    #[test]
    fn login_csrf_validation_checks_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, "__Host-tee_login=token".parse().unwrap());
        assert!(validate_login_csrf(&headers, "token").is_ok());
        assert!(matches!(
            validate_login_csrf(&headers, "wrong"),
            Err(AppError::CsrfViolation)
        ));
    }

    #[tokio::test]
    async fn session_validation_returns_principal() {
        let Some(pool) = test_pool().await else {
            eprintln!("DATABASE_URL not set; skipping");
            return;
        };
        let salt = SaltString::from_b64("c29tZXNhbHQxMjM0").unwrap();
        let hash = Argon2::default()
            .hash_password("password".as_bytes(), &salt)
            .unwrap()
            .to_string();
        let user_id = insert_user(&pool, "spec-auth@example.com", &hash, true).await;
        let session_token = "session-token";
        let session_hash = hash_token(session_token);
        let session_id = insert_session(
            &pool,
            user_id,
            &session_hash,
            Utc::now() + chrono::Duration::hours(4),
        )
        .await;
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            format!("__Host-tee_session={session_token}")
                .parse()
                .unwrap(),
        );
        let state = auth_state(pool.clone());
        let auth = super::build_auth_context(&state, &headers)
            .await
            .expect("auth context");
        assert!(auth.principal.is_some());
        assert!(auth.csrf_token.is_some());
        cleanup_auth(&pool, user_id, session_id).await;
    }

    #[tokio::test]
    async fn idle_timeout_revokes_session() {
        let Some(pool) = test_pool().await else {
            eprintln!("DATABASE_URL not set; skipping");
            return;
        };
        let salt = SaltString::from_b64("c29tZXNhbHQxMjM0").unwrap();
        let hash = Argon2::default()
            .hash_password("password".as_bytes(), &salt)
            .unwrap()
            .to_string();
        let user_id = insert_user(&pool, "spec-auth-idle@example.com", &hash, true).await;
        let session_token = "idle-token";
        let session_hash = hash_token(session_token);
        let session_id = insert_session(
            &pool,
            user_id,
            &session_hash,
            Utc::now() + chrono::Duration::hours(4),
        )
        .await;
        let expired = Utc::now() - chrono::Duration::hours(2);
        sqlx::query("UPDATE sessions SET last_seen_at = $1 WHERE id = $2")
            .bind(expired)
            .bind(session_id)
            .execute(&pool)
            .await
            .expect("update last_seen_at");
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            format!("__Host-tee_session={session_token}")
                .parse()
                .unwrap(),
        );
        let state = auth_state(pool.clone());
        let auth = super::build_auth_context(&state, &headers)
            .await
            .expect("auth context");
        assert!(auth.principal.is_none());
        let revoked_at: Option<chrono::DateTime<Utc>> =
            sqlx::query_scalar("SELECT revoked_at FROM sessions WHERE id = $1")
                .bind(session_id)
                .fetch_one(&pool)
                .await
                .expect("fetch revoked_at");
        assert!(revoked_at.is_some());
        cleanup_auth(&pool, user_id, session_id).await;
    }

    #[tokio::test]
    async fn logout_revokes_session() {
        let Some(pool) = test_pool().await else {
            eprintln!("DATABASE_URL not set; skipping");
            return;
        };
        let salt = SaltString::from_b64("c29tZXNhbHQxMjM0").unwrap();
        let hash = Argon2::default()
            .hash_password("password".as_bytes(), &salt)
            .unwrap()
            .to_string();
        let user_id = insert_user(&pool, "spec-auth-logout@example.com", &hash, true).await;
        let session_id = insert_session(
            &pool,
            user_id,
            "logout-hash",
            Utc::now() + chrono::Duration::hours(2),
        )
        .await;
        auth_repo::revoke_session(&pool, session_id)
            .await
            .expect("revoke session");
        let revoked_at: Option<chrono::DateTime<Utc>> =
            sqlx::query_scalar("SELECT revoked_at FROM sessions WHERE id = $1")
                .bind(session_id)
                .fetch_one(&pool)
                .await
                .expect("fetch revoked_at");
        assert!(revoked_at.is_some());
        cleanup_auth(&pool, user_id, session_id).await;
    }
}
