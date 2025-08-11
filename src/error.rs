use derive_more::{Display, Error, From};

#[derive(Debug, Display, Error, From)]
pub enum InitializationError {
    #[display("failed to load configuration from env: {_0}")]
    Config(envconfig::Error),

    #[display("invalid BIND_HOST '{value}': {source}")]
    InvalidBindHost {
        value: String,
        source: std::net::AddrParseError,
    },

    #[display("IO error: {_0}")]
    IoError(std::io::Error),
}
