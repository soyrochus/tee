mod app;
mod data;
mod domain;
mod interface;
mod ops;

use anyhow::Context;
use interface::i18n::{Translator, DEFAULT_LOCALE, REQUIRED_LOCALES};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cfg = ops::config::Config::from_env().context("reading config")?;
    ops::observability::init(&cfg.log_level);

    let pool = data::db::new_pool(&cfg.database_url)
        .await
        .context("connecting to database")?;

    // Dev/test convenience: run migrations on startup.
    // In production, prefer a controlled migration step in CI/CD.
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("running migrations")?;

    let translator = Translator::load_from_disk("locales", DEFAULT_LOCALE, REQUIRED_LOCALES)
        .context("loading translation bundles")?;

    let auth_settings = interface::state::AuthSettings::new(
        cfg.session_lifetime,
        cfg.session_idle_timeout,
        cfg.remember_me_lifetime,
    );
    let router = interface::http::build_router(pool, auth_settings, translator);

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr)
        .await
        .context("binding TCP listener")?;

    tracing::info!("listening on {}", cfg.bind_addr);

    axum::serve(listener, router)
        .await
        .context("serving HTTP")?;
    Ok(())
}
