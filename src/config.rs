use crate::error::InitializationError;
use envconfig::Envconfig;
use std::net::SocketAddr;

#[derive(Clone, Debug, Envconfig)]
pub struct AppConfig {
    #[envconfig(from = "BIND_HOST", default = "0.0.0.0:3000")]
    pub bind_host: String,
}

impl AppConfig {
    pub fn load() -> Result<Self, InitializationError> {
        AppConfig::init_from_env().map_err(InitializationError::from)
    }

    pub fn bind_addr(&self) -> Result<SocketAddr, InitializationError> {
        self.bind_host
            .parse()
            .map_err(|e| InitializationError::InvalidBindHost {
                value: self.bind_host.clone(),
                source: e,
            })
    }
}
