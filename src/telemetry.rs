use tracing_subscriber::EnvFilter;

const DEFAULT_ENV_FILTER: &str = "info";

pub fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_ENV_FILTER));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
