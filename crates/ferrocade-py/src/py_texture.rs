//! `ferrocade.Texture`: a loaded image, shared by every sprite that uses it.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use ferrocade::texture::{self, TextureData, TextureRef};
use pyo3::prelude::*;

use crate::errors::IntoPyResult;
use crate::py_filter::PyFilter;

#[pyclass(frozen, name = "Texture", module = "ferrocade")]
pub struct PyTexture {
    data: Arc<TextureData>,
    filter: Option<ferrocade::texture::Filter>,
}

impl PyTexture {
    pub fn texture_ref(&self) -> TextureRef {
        TextureRef {
            data: self.data.clone(),
            filter: self.filter,
        }
    }

    pub fn from_ref(texture: &TextureRef) -> Self {
        Self {
            data: texture.data.clone(),
            filter: texture.filter,
        }
    }

    /// Load from a path through the shared cache (no filter override).
    pub fn load(path: PathBuf) -> PyResult<Self> {
        Ok(Self {
            data: texture::load(&path).py()?,
            filter: None,
        })
    }
}

#[pymethods]
impl PyTexture {
    #[new]
    #[pyo3(signature = (path, *, filter=None))]
    fn new(path: PathBuf, filter: Option<PyFilter>) -> PyResult<Self> {
        Ok(Self {
            data: texture::load(&path).py()?,
            filter: filter.map(Into::into),
        })
    }

    /// Width in pixels.
    #[getter]
    fn width(&self) -> u32 {
        self.data.width
    }

    /// Height in pixels.
    #[getter]
    fn height(&self) -> u32 {
        self.data.height
    }

    /// Canonical absolute path of the image file.
    #[getter]
    fn path(&self) -> String {
        self.data.path.to_string_lossy().into_owned()
    }

    /// Filter override, or `None` to use the window's filter.
    #[getter]
    fn filter(&self) -> Option<PyFilter> {
        self.filter.map(Into::into)
    }

    fn __eq__(&self, other: PyRef<'_, Self>) -> bool {
        self.data.id == other.data.id && self.filter == other.filter
    }

    fn __hash__(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.data.id.hash(&mut h);
        self.filter.hash(&mut h);
        h.finish()
    }

    fn __repr__(&self) -> String {
        let filter = match self.filter {
            Some(ferrocade::texture::Filter::Nearest) => ", filter=NEAREST".to_string(),
            Some(ferrocade::texture::Filter::Linear) => ", filter=LINEAR".to_string(),
            None => String::new(),
        };
        format!("Texture({:?}{filter})", self.path())
    }
}
