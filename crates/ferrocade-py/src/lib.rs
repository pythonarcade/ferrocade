//! Python bindings for Ferrocade, built as the `ferrocade._ferrocade` extension module.

mod errors;
mod py_filter;
mod py_shapes;
mod py_sprite;
mod py_sprite_list;
mod py_texture;
mod py_window;

use pyo3::prelude::*;

#[pymodule]
fn _ferrocade(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Lets users enable engine diagnostics with RUST_LOG; ignore "already initialized".
    let _ = env_logger::try_init();
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    py_filter::register(m)?;
    m.add_class::<py_window::PyWindow>()?;
    m.add_class::<py_texture::PyTexture>()?;
    m.add_class::<py_sprite::PySprite>()?;
    m.add_class::<py_sprite_list::PySpriteList>()?;
    m.add_class::<py_shapes::PyRectangleSprite>()?;
    m.add_class::<py_shapes::PyCircleSprite>()?;
    m.add_function(wrap_pyfunction!(_debug_texture_disk_reads, m)?)?;
    Ok(())
}

/// Number of image files read from disk so far (private; used by tests to verify the cache).
#[pyfunction]
fn _debug_texture_disk_reads() -> u64 {
    ferrocade::texture::disk_reads()
}
