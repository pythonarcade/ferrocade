//! `ferrocade.Sprite`: a handle to sprite data stored in the Rust core.
//!
//! Every property reads or writes the central store directly; nothing is kept as Python floats.

use std::path::PathBuf;

use ferrocade::color;
use ferrocade::store::{self, SpriteId};
use pyo3::exceptions::{PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use pyo3::{PyTraverseError, PyVisit};

use crate::errors::IntoPyResult;
use crate::py_collider::PyCollider;
use crate::py_sprite_list::PySpriteList;
use crate::py_texture::PyTexture;

#[pyclass(subclass, weakref, name = "Sprite", module = "ferrocade")]
pub struct PySprite {
    /// Set by `__init__`; `None` if a subclass forgot to call `super().__init__()`.
    id: Option<SpriteId>,
    /// The attached collider's Python object (kept alive while attached).
    collider: Option<Py<PyCollider>>,
}

impl PySprite {
    /// An uninitialized sprite; subclasses (shape sprites) fill in the id in their `__init__`.
    pub fn empty() -> Self {
        Self {
            id: None,
            collider: None,
        }
    }

    /// Attach a newly created store sprite (used by shape-sprite subclasses).
    pub fn set_id(&mut self, id: SpriteId) -> PyResult<()> {
        if self.id.is_some() {
            return Err(PyRuntimeError::new_err("sprite was already initialized"));
        }
        self.id = Some(id);
        Ok(())
    }

    pub fn id(&self) -> PyResult<SpriteId> {
        self.id.ok_or_else(|| {
            PyRuntimeError::new_err(
                "Sprite.__init__() was not called; call super().__init__(...) in your subclass",
            )
        })
    }

    fn read<T>(&self, f: impl FnOnce(&store::SpriteState) -> T) -> PyResult<T> {
        let id = self.id()?;
        let store = store::global();
        Ok(f(store.sprite(id).py()?))
    }
}

/// Accept a `Texture` or a path (str / os.PathLike).
fn texture_arg(obj: &Bound<'_, PyAny>) -> PyResult<PyTexture> {
    if let Ok(texture) = obj.extract::<PyRef<'_, PyTexture>>() {
        return Ok(PyTexture::from_ref(&texture.texture_ref()));
    }
    match obj.extract::<PathBuf>() {
        Ok(path) => PyTexture::load(path),
        Err(_) => Err(PyTypeError::new_err(format!(
            "texture must be a Texture or a file path, not {}",
            obj.get_type().name()?
        ))),
    }
}

#[pymethods]
impl PySprite {
    // Accept anything here so Python subclasses can pass their own arguments; the real
    // initialization happens in __init__.
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn new(_args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>) -> Self {
        Self::empty()
    }

    #[pyo3(signature = (texture, center_x=0.0, center_y=0.0, scale=1.0))]
    fn __init__(
        &mut self,
        texture: &Bound<'_, PyAny>,
        center_x: f64,
        center_y: f64,
        scale: f64,
    ) -> PyResult<()> {
        if self.id.is_some() {
            return Err(PyRuntimeError::new_err(
                "Sprite.__init__() was already called",
            ));
        }
        let texture = texture_arg(texture)?.texture_ref();
        let id = store::global()
            .create_sprite(texture, center_x, center_y, scale)
            .py()?;
        self.id = Some(id);
        Ok(())
    }

    #[getter]
    fn center_x(&self) -> PyResult<f64> {
        self.read(|s| s.center_x)
    }

    #[setter]
    fn set_center_x(&self, value: f64) -> PyResult<()> {
        store::global().set_center_x(self.id()?, value).py()
    }

    #[getter]
    fn center_y(&self) -> PyResult<f64> {
        self.read(|s| s.center_y)
    }

    #[setter]
    fn set_center_y(&self, value: f64) -> PyResult<()> {
        store::global().set_center_y(self.id()?, value).py()
    }

    /// `(center_x, center_y)`; setting it updates both in one call.
    #[getter]
    fn position(&self) -> PyResult<(f64, f64)> {
        self.read(|s| (s.center_x, s.center_y))
    }

    #[setter]
    fn set_position(&self, value: (f64, f64)) -> PyResult<()> {
        store::global()
            .set_position(self.id()?, value.0, value.1)
            .py()
    }

    #[getter]
    fn scale(&self) -> PyResult<f64> {
        self.read(|s| s.scale)
    }

    #[setter]
    fn set_scale(&self, value: f64) -> PyResult<()> {
        store::global().set_scale(self.id()?, value).py()
    }

    /// Rotation in degrees (stored; rotation is not drawn yet).
    #[getter]
    fn angle(&self) -> PyResult<f64> {
        self.read(|s| s.angle)
    }

    #[setter]
    fn set_angle(&self, value: f64) -> PyResult<()> {
        store::global().set_angle(self.id()?, value).py()
    }

    #[getter]
    fn texture(&self) -> PyResult<PyTexture> {
        self.read(|s| PyTexture::from_ref(&s.texture))
    }

    #[setter]
    fn set_texture(&self, value: PyRef<'_, PyTexture>) -> PyResult<()> {
        let id = self.id()?;
        let mut store = store::global();
        if store.sprite(id).py()?.is_shape() {
            return Err(PyTypeError::new_err(
                "shape sprites use a built-in texture; it can't be replaced",
            ));
        }
        store.set_texture(id, value.texture_ref()).py()
    }

    /// Horizontal velocity in pixels per second (applied by `SpriteList.update`).
    #[getter]
    fn velocity_x(&self) -> PyResult<f64> {
        self.read(|s| s.velocity_x)
    }

    #[setter]
    fn set_velocity_x(&self, value: f64) -> PyResult<()> {
        store::global().set_velocity_x(self.id()?, value).py()
    }

    /// Vertical velocity in pixels per second (+y is up).
    #[getter]
    fn velocity_y(&self) -> PyResult<f64> {
        self.read(|s| s.velocity_y)
    }

    #[setter]
    fn set_velocity_y(&self, value: f64) -> PyResult<()> {
        store::global().set_velocity_y(self.id()?, value).py()
    }

    /// `(velocity_x, velocity_y)`; setting it updates both in one call.
    #[getter]
    fn velocity(&self) -> PyResult<(f64, f64)> {
        self.read(|s| (s.velocity_x, s.velocity_y))
    }

    #[setter]
    fn set_velocity(&self, value: (f64, f64)) -> PyResult<()> {
        store::global()
            .set_velocity(self.id()?, value.0, value.1)
            .py()
    }

    /// Rotation speed in degrees per second (positive is counter-clockwise).
    #[getter]
    fn angular_velocity(&self) -> PyResult<f64> {
        self.read(|s| s.angular_velocity)
    }

    #[setter]
    fn set_angular_velocity(&self, value: f64) -> PyResult<()> {
        store::global().set_angular_velocity(self.id()?, value).py()
    }

    /// Tint as (r, g, b, a), 0-255. White (the default) draws the texture unchanged.
    #[getter]
    fn color(&self) -> PyResult<(u8, u8, u8, u8)> {
        self.read(|s| {
            let [r, g, b, a] = s.color;
            (r, g, b, a)
        })
    }

    #[setter]
    fn set_color(&self, value: Vec<i64>) -> PyResult<()> {
        let rgba = color::parse_rgba(&value).py()?;
        store::global().set_color(self.id()?, rgba).py()
    }

    /// Drawn width in pixels: base size (or texture width) × |scale|.
    #[getter]
    fn width(&self) -> PyResult<f64> {
        self.read(|s| s.drawn_size().0)
    }

    /// Drawn height in pixels: base size (or texture height) × |scale|.
    #[getter]
    fn height(&self) -> PyResult<f64> {
        self.read(|s| s.drawn_size().1)
    }

    /// The attached collider, or `None`. Assign a collider to attach it (replacing any
    /// previous one), or `None` to detach. A collider can be attached to one sprite at a time.
    #[getter]
    fn collider(&self, py: Python<'_>) -> Option<Py<PyCollider>> {
        self.collider.as_ref().map(|c| c.clone_ref(py))
    }

    #[setter]
    fn set_collider(
        slf: &Bound<'_, Self>,
        collider: Option<Bound<'_, PyCollider>>,
    ) -> PyResult<()> {
        let sprite_id = slf.borrow().id()?;
        let collider_id = match &collider {
            Some(c) => Some(c.borrow().id()?),
            None => None,
        };
        store::global()
            .attach_collider(sprite_id, collider_id)
            .py()?;
        let previous = slf.borrow_mut().collider.take();
        if let Some(old) = previous {
            let old = old.bind(slf.py());
            if collider.as_ref().is_none_or(|c| !c.is(old)) {
                old.borrow_mut().set_owner(None)?;
            }
        }
        if let Some(c) = &collider {
            c.borrow_mut().set_owner(Some(slf))?;
        }
        slf.borrow_mut().collider = collider.map(Bound::unbind);
        Ok(())
    }

    /// Whether this sprite's collider touches `other`'s (touching counts). `False` if either has
    /// no collider, and for the sprite itself.
    fn collides_with(&self, other: PyRef<'_, PySprite>) -> PyResult<bool> {
        let (a, b) = (self.id()?, other.id()?);
        store::global().collides(a, b).py()
    }

    /// The sprites in `sprites` whose colliders touch this sprite's, in list order (never this
    /// sprite itself). One engine call; the result is a new list.
    fn collides_with_list<'py>(
        &self,
        py: Python<'py>,
        sprites: PyRef<'py, PySpriteList>,
    ) -> PyResult<Bound<'py, PyList>> {
        let slots = store::global()
            .sprite_vs_list(self.id()?, sprites.list_id())
            .py()?;
        let refs = sprites.refs();
        PyList::new(py, slots.iter().map(|&i| refs[i as usize].clone_ref(py)))
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some(c) = &self.collider {
            visit.call(c)?;
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        self.collider = None;
    }
}

impl Drop for PySprite {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            store::global().destroy_sprite(id);
        }
    }
}
