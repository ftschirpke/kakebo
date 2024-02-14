use thiserror::Error;

#[derive(Debug, Error)]
pub enum KakeboError {
    #[error("Invalid value: {0}")]
    Parse(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Deserialization error: {0}")]
    Deserialization(#[from] toml::de::Error),
}
