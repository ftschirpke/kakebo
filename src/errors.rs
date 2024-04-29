use thiserror::Error;

#[derive(Debug, Error)]
pub enum KakeboError {
    #[error("Invalid value: {0}")]
    Parse(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Decryption error: {0}")]
    Decryption(#[from] age::DecryptError),
    #[error("Encryption error: {0}")]
    Encryption(#[from] age::EncryptError),
    #[error("Deserialization error: {0}")]
    Deserialization(#[from] toml::de::Error),
}
