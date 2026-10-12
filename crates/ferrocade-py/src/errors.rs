//! Map engine errors to Python exceptions.

use ferrocade::Error;
use pyo3::PyErr;
use pyo3::exceptions::{PyFileNotFoundError, PyOSError, PyRuntimeError, PyValueError};

pub fn to_pyerr(e: Error) -> PyErr {
    let msg = e.to_string();
    match e {
        Error::NotFound { .. } => PyFileNotFoundError::new_err(msg),
        Error::Io { .. } => PyOSError::new_err(msg),
        Error::Decode { .. } | Error::Unsupported { .. } | Error::InvalidArgument(_) => {
            PyValueError::new_err(msg)
        }
        Error::NotInFrame { .. } | Error::Gpu(_) | Error::EventLoop(_) => {
            PyRuntimeError::new_err(msg)
        }
    }
}

/// `?`-friendly conversion for engine results.
pub trait IntoPyResult<T> {
    fn py(self) -> pyo3::PyResult<T>;
}

impl<T> IntoPyResult<T> for ferrocade::Result<T> {
    fn py(self) -> pyo3::PyResult<T> {
        self.map_err(to_pyerr)
    }
}
