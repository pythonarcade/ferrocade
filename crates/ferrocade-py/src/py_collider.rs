//! `ferrocade.CircleCollider`, `BoxCollider`, `AABBCollider`: collision shapes attached to sprites.
//!
//! Colliders are separate objects (engine spec §11). The data lives in the core store; these
//! classes hold its id. A sprite keeps its collider alive; the collider refers back to its
//! sprite only weakly, so no reference cycle forms.

use ferrocade::collision::{ColliderId, Shape};
use ferrocade::store;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple, PyWeakrefReference};

use crate::errors::IntoPyResult;
use crate::py_sprite::PySprite;

/// Base class of all colliders (not constructed directly).
#[pyclass(subclass, weakref, name = "Collider", module = "ferrocade")]
pub struct PyCollider {
    id: Option<ColliderId>,
    /// The sprite this collider is attached to (weak, to avoid a cycle).
    sprite: Option<Py<PyWeakrefReference>>,
}

impl PyCollider {
    fn empty() -> Self {
        Self {
            id: None,
            sprite: None,
        }
    }

    pub fn id(&self) -> PyResult<ColliderId> {
        self.id.ok_or_else(|| {
            PyRuntimeError::new_err(
                "collider is not initialized; call super().__init__(...) in your subclass",
            )
        })
    }

    fn init(&mut self, shape: Shape, offset: (f64, f64)) -> PyResult<()> {
        let mut store = store::global();
        if let Some(old) = self.id.take() {
            store.destroy_collider(old);
        }
        self.id = Some(store.create_collider(shape, offset).py()?);
        Ok(())
    }

    /// Record (or clear) the sprite this collider is attached to.
    pub fn set_owner(&mut self, sprite: Option<&Bound<'_, PySprite>>) -> PyResult<()> {
        self.sprite = match sprite {
            Some(s) => Some(PyWeakrefReference::new(s)?.unbind()),
            None => None,
        };
        Ok(())
    }

    fn shape(&self) -> PyResult<Shape> {
        Ok(store::global().collider(self.id()?).py()?.shape)
    }

    fn set_shape(&self, shape: Shape) -> PyResult<()> {
        store::global().set_collider_shape(self.id()?, shape).py()
    }
}

#[pymethods]
impl PyCollider {
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn new(_args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>) -> Self {
        Self::empty()
    }

    /// Center relative to the sprite center, in unscaled sprite pixels.
    #[getter]
    fn offset(&self) -> PyResult<(f64, f64)> {
        Ok(store::global().collider(self.id()?).py()?.offset)
    }

    #[setter]
    fn set_offset(&self, offset: (f64, f64)) -> PyResult<()> {
        store::global().set_collider_offset(self.id()?, offset).py()
    }

    /// The sprite this collider is attached to, or `None`.
    #[getter]
    fn sprite<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyAny>> {
        self.sprite.as_ref().and_then(|w| w.bind(py).upgrade())
    }
}

impl Drop for PyCollider {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            store::global().destroy_collider(id);
        }
    }
}

fn size_repr(v: f64) -> String {
    format!("{v}")
}

/// A circle collider. It follows the sprite's position and scale; its offset rotates with
/// the sprite.
#[pyclass(extends = PyCollider, subclass, name = "CircleCollider", module = "ferrocade")]
pub struct PyCircleCollider {}

#[pymethods]
impl PyCircleCollider {
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn new(
        _args: &Bound<'_, PyTuple>,
        _kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyClassInitializer<Self> {
        PyClassInitializer::from(PyCollider::empty()).add_subclass(Self {})
    }

    #[pyo3(signature = (radius, *, offset=(0.0, 0.0)))]
    fn __init__(slf: PyRefMut<'_, Self>, radius: f64, offset: (f64, f64)) -> PyResult<()> {
        slf.into_super().init(Shape::Circle { radius }, offset)
    }

    /// Radius in unscaled sprite pixels.
    #[getter]
    fn radius(slf: PyRef<'_, Self>) -> PyResult<f64> {
        match slf.as_super().shape()? {
            Shape::Circle { radius } => Ok(radius),
            _ => unreachable!("a CircleCollider holds a circle"),
        }
    }

    #[setter]
    fn set_radius(slf: PyRef<'_, Self>, radius: f64) -> PyResult<()> {
        slf.as_super().set_shape(Shape::Circle { radius })
    }

    fn __repr__(slf: PyRef<'_, Self>) -> PyResult<String> {
        let r = Self::radius(slf)?;
        Ok(format!("CircleCollider(radius={})", size_repr(r)))
    }
}

/// Width and height of a box shape.
fn box_size(shape: Shape) -> (f64, f64) {
    match shape {
        Shape::Box { width, height } | Shape::Aabb { width, height } => (width, height),
        Shape::Circle { .. } => unreachable!("a box collider holds a box"),
    }
}

macro_rules! box_collider {
    ($rust:ident, $py:literal, $variant:ident, $doc:literal) => {
        #[doc = $doc]
        #[pyclass(extends = PyCollider, subclass, name = $py, module = "ferrocade")]
        pub struct $rust {}

        #[pymethods]
        impl $rust {
            #[new]
            #[pyo3(signature = (*_args, **_kwargs))]
            fn new(
                _args: &Bound<'_, PyTuple>,
                _kwargs: Option<&Bound<'_, PyDict>>,
            ) -> PyClassInitializer<Self> {
                PyClassInitializer::from(PyCollider::empty()).add_subclass(Self {})
            }

            #[pyo3(signature = (width, height, *, offset=(0.0, 0.0)))]
            fn __init__(
                slf: PyRefMut<'_, Self>,
                width: f64,
                height: f64,
                offset: (f64, f64),
            ) -> PyResult<()> {
                slf.into_super()
                    .init(Shape::$variant { width, height }, offset)
            }

            /// Width in unscaled sprite pixels.
            #[getter]
            fn width(slf: PyRef<'_, Self>) -> PyResult<f64> {
                Ok(box_size(slf.as_super().shape()?).0)
            }

            #[setter]
            fn set_width(slf: PyRef<'_, Self>, width: f64) -> PyResult<()> {
                let (_, height) = box_size(slf.as_super().shape()?);
                slf.as_super().set_shape(Shape::$variant { width, height })
            }

            /// Height in unscaled sprite pixels.
            #[getter]
            fn height(slf: PyRef<'_, Self>) -> PyResult<f64> {
                Ok(box_size(slf.as_super().shape()?).1)
            }

            #[setter]
            fn set_height(slf: PyRef<'_, Self>, height: f64) -> PyResult<()> {
                let (width, _) = box_size(slf.as_super().shape()?);
                slf.as_super().set_shape(Shape::$variant { width, height })
            }

            fn __repr__(slf: PyRef<'_, Self>) -> PyResult<String> {
                let (w, h) = box_size(slf.as_super().shape()?);
                Ok(format!(
                    "{}(width={}, height={})",
                    $py,
                    size_repr(w),
                    size_repr(h)
                ))
            }
        }
    };
}

box_collider!(
    PyBoxCollider,
    "BoxCollider",
    Box,
    "A box collider that rotates with its sprite (its offset rotates too)."
);
box_collider!(
    PyAabbCollider,
    "AABBCollider",
    Aabb,
    "An axis-aligned box collider: it never rotates, and neither does its offset."
);
