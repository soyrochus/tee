use anyhow::Context;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub log_level: String,
    pub session_lifetime: std::time::Duration,
    pub session_idle_timeout: std::time::Duration,
    pub remember_me_lifetime: std::time::Duration,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let database_url = std::env::var("DATABASE_URL").context("missing env var DATABASE_URL")?;
        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        let session_lifetime = read_duration_hours("SESSION_LIFETIME_HOURS", 12)?;
        let session_idle_timeout = read_duration_minutes("SESSION_IDLE_TIMEOUT_MINUTES", 45)?;
        let remember_me_lifetime = read_duration_days("SESSION_REMEMBER_ME_DAYS", 30)?;
        Ok(Self {
            bind_addr,
            database_url,
            log_level,
            session_lifetime,
            session_idle_timeout,
            remember_me_lifetime,
        })
    }
}

fn read_duration_hours(
    var: &str,
    default_hours: u64,
) -> Result<std::time::Duration, anyhow::Error> {
    let hours = std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_hours);
    Ok(std::time::Duration::from_secs(hours * 60 * 60))
}

fn read_duration_minutes(
    var: &str,
    default_minutes: u64,
) -> Result<std::time::Duration, anyhow::Error> {
    let minutes = std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_minutes);
    Ok(std::time::Duration::from_secs(minutes * 60))
}

fn read_duration_days(var: &str, default_days: u64) -> Result<std::time::Duration, anyhow::Error> {
    let days = std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_days);
    Ok(std::time::Duration::from_secs(days * 24 * 60 * 60))
}
