use thiserror::Error;

#[derive(Debug, Error)]
pub enum InteropError {
    #[error("packed value spans slot boundary: offset={offset}, bytes={bytes}")]
    PackedSlotOverflow { offset: usize, bytes: usize },
    #[error("invalid boolean value: {0}")]
    InvalidBool(u64),
    #[error("invalid signed value encoding")]
    InvalidSignedEncoding,
}

pub type Result<T> = std::result::Result<T, InteropError>;
