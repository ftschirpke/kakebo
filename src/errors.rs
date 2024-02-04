use thiserror::Error;

#[derive(Debug, Error)]
pub enum KakeboError {
    #[error("Invalid value: {0}")]
    Parse(String),
}

