use std::time::Duration;

use crate::interface::i18n::Translator;

#[derive(Clone)]
pub struct AuthSettings {
    pub session_lifetime: Duration,
    pub session_idle_timeout: Duration,
    pub remember_me_lifetime: Duration,
}

impl AuthSettings {
    pub fn new(
        session_lifetime: Duration,
        session_idle_timeout: Duration,
        remember_me_lifetime: Duration,
    ) -> Self {
        Self {
            session_lifetime,
            session_idle_timeout,
            remember_me_lifetime,
        }
    }
}

#[derive(Clone)]
pub struct I18nState {
    pub translator: Translator,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub auth: AuthSettings,
    pub i18n: I18nState,
}
