//! SPIKE: approach A experiment. Exercises the Ferrocade core from inside a Pyodide wheel.

use std::path::Path;

use ferrocade::store::SpriteStore;
use ferrocade::texture::{self, TextureRef};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Decode an image through the core's texture cache; returns (width, height).
#[pyfunction]
fn texture_size(path: &str, data: &[u8]) -> PyResult<(u32, u32)> {
    let t = texture::load_from_bytes(Path::new(path), data)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok((t.width, t.height))
}

/// Create 3 sprites in a list, remove one; returns the list length (expected 2).
#[pyfunction]
fn store_smoke(path: &str, data: &[u8]) -> PyResult<usize> {
    let err = |e: ferrocade::Error| PyValueError::new_err(e.to_string());
    let data = texture::load_from_bytes(Path::new(path), data).map_err(err)?;
    let mut store = SpriteStore::new();
    let list = store.create_list();
    let mut ids = Vec::new();
    for i in 0..3 {
        let tex = TextureRef { data: data.clone(), filter: None };
        let id = store.create_sprite(tex, i as f64, 0.0, 1.0).map_err(err)?;
        store.append(list, id).map_err(err)?;
        ids.push(id);
    }
    store.remove(list, ids[1]).map_err(err)?;
    Ok(store.list(list).map_err(err)?.sprites().len())
}

/// A2 only: touch wgpu so its web code is linked into the wheel.
#[cfg(feature = "render")]
#[pyfunction]
fn renderer_smoke() -> String {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    format!("wgpu instance created: {instance:?}")
}

#[pymodule]
fn ferrocade_a1(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(texture_size, m)?)?;
    m.add_function(wrap_pyfunction!(store_smoke, m)?)?;
    #[cfg(feature = "render")]
    m.add_function(wrap_pyfunction!(renderer_smoke, m)?)?;
    Ok(())
}
