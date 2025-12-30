use tracing_subscriber::{fmt, EnvFilter};

pub fn init(log_level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
    fmt().with_env_filter(filter).init();
}
