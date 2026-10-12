//! `ferrocade.RectangleSprite` and `ferrocade.CircleSprite`: solid-color sprites, no image needed.
//!
//! Both are kinds of `Sprite` drawn from a shared built-in shape texture tinted with their color,
//! so any mix of them in a sprite list batches into one draw call.

use ferrocade::color;
use ferrocade::store;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

use crate::errors::IntoPyResult;
use crate::py_filter::PyFilter;
use crate::py_sprite::PySprite;

#[pyclass(extends = PySprite, subclass, name = "RectangleSprite", module = "ferrocade")]
pub struct PyRectangleSprite {}

#[pymethods]
impl PyRectangleSprite {
    // Accept anything so Python subclasses can pass their own arguments; see __init__.
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn new(
        _args: &Bound<'_, PyTuple>,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PySprite::empty()).add_subclass(Self {})
    }

    #[pyo3(signature = (width, height, color, center_x=0.0, center_y=0.0))]
    fn __init__(
        slf: PyRefMut<'_, Self>,
        width: f64,
        height: f64,
        color: Vec<i64>,
        center_x: f64,
        center_y: f64,
    ) -> PyResult<()> {
        let rgba = color::parse_rgba(&color).py()?;
        let id = store::global()
            .create_rectangle(width, height, rgba, center_x, center_y)
            .py()?;
        slf.into_super().set_id(id)
    }
}

#[pyclass(extends = PySprite, subclass, name = "CircleSprite", module = "ferrocade")]
pub struct PyCircleSprite {}

#[pymethods]
impl PyCircleSprite {
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn new(
        _args: &Bound<'_, PyTuple>,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PySprite::empty()).add_subclass(Self {})
    }

    #[pyo3(signature = (radius, color, center_x=0.0, center_y=0.0, *, filter=None))]
    fn __init__(
        slf: PyRefMut<'_, Self>,
        radius: f64,
        color: Vec<i64>,
        center_x: f64,
        center_y: f64,
        filter: Option<PyFilter>,
    ) -> PyResult<()> {
        let rgba = color::parse_rgba(&color).py()?;
        let id = store::global()
            .create_circle(radius, rgba, center_x, center_y, filter.map(Into::into))
            .py()?;
        slf.into_super().set_id(id)
    }

    /// Drawn radius in pixels: radius × |scale|.
    #[getter]
    fn radius(slf: PyRef<'_, Self>) -> PyResult<f64> {
        let id = slf.as_super().id()?;
        let store = store::global();
        Ok(store.sprite(id).py()?.drawn_size().0 / 2.0)
    }
}
