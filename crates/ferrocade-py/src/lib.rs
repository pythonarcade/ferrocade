//! Python bindings for Ferrocade, built as the `ferrocade._ferrocade` extension module.

mod errors;
mod py_filter;

use pyo3::prelude::*;

#[pymodule]
fn _ferrocade(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Lets users enable engine diagnostics with RUST_LOG; ignore "already initialized".
    let _ = env_logger::try_init();
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    py_filter::register(m)?;
    Ok(())
}
