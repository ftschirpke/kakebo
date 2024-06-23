use thiserror::Error;

#[derive(Debug, Error)]
pub enum KakeboError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("(De)Compression error: {0}")]
    Compression(#[from] lz4_flex::frame::Error),
    #[error("LZ4 Decryption error: {0}")]
    Decryption(#[from] age::DecryptError),
    #[error("LZ4 Encryption error: {0}")]
    Encryption(#[from] age::EncryptError),
    #[error("RMP decode error: {0}")]
    RmpDecode(#[from] rmp_serde::decode::Error),
    #[error("RMP encode error: {0}")]
    RmpEncode(#[from] rmp_serde::encode::Error),
    #[error("RON Deserialization error: {0}")]
    RonDeserialization(#[from] ron::de::SpannedError),
    #[error("RON Serialization error: {0}")]
    RonSerialization(#[from] ron::Error),
    #[error("Toml Serialization error: {0}")]
    TomlSerialization(#[from] toml::ser::Error),
    #[error("Toml Deserialization error: {0}")]
    TomlDeserialization(#[from] toml::de::Error),
    #[error("Inquire error: {0}")]
    Inquire(#[from] inquire::error::InquireError),
    #[error("Expense creation aborted")]
    ExpenseCreationAborted,
}
