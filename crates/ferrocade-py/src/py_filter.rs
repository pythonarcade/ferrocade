//! `ferrocade.Filter`: texture filtering mode (no default anywhere in the API).

use ferrocade::texture::Filter;
use pyo3::prelude::*;

#[pyclass(eq, frozen, hash, from_py_object, name = "Filter", module = "ferrocade")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum PyFilter {
    /// Crisp pixels with no smoothing (pixel art).
    NEAREST,
    /// Smooth interpolation (photos, scaled or rotated art).
    LINEAR,
}

impl From<PyFilter> for Filter {
    fn from(f: PyFilter) -> Self {
        match f {
            PyFilter::NEAREST => Filter::Nearest,
            PyFilter::LINEAR => Filter::Linear,
        }
    }
}

impl From<Filter> for PyFilter {
    fn from(f: Filter) -> Self {
        match f {
            Filter::Nearest => PyFilter::NEAREST,
            Filter::Linear => PyFilter::LINEAR,
        }
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFilter>()?;
    m.add("NEAREST", PyFilter::NEAREST)?;
    m.add("LINEAR", PyFilter::LINEAR)?;
    Ok(())
}
