use thiserror::Error;

#[derive(Error, Debug)]
pub enum GoCoreError {
    #[error("KataGo process not running")]
    ProcessNotRunning,

    #[error("KataGo process pipe closed")]
    PipeClosed,

    #[error("KataGo command timed out: {0}")]
    Timeout(String),

    #[error("KataGo rejected command: {0}")]
    Rejected(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, GoCoreError>;
