//! Engine error type. The Python bindings map each variant to a Python exception.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("file not found: '{}'", path.display())]
    NotFound { path: PathBuf },

    #[error("could not read '{}': {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not decode image '{}': {reason}", path.display())]
    Decode { path: PathBuf, reason: String },

    #[error("unsupported file '{}': {reason}", path.display())]
    Unsupported { path: PathBuf, reason: String },

    #[error("{op}() can only be called inside on_draw()")]
    NotInFrame { op: &'static str },

    #[error("{0}")]
    InvalidArgument(String),

    #[error("graphics error: {0}")]
    Gpu(String),

    #[error("event loop error: {0}")]
    EventLoop(String),
}

pub type Result<T> = std::result::Result<T, Error>;
