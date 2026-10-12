//! `ferrocade.SpriteList`: an ordered batch of sprites, drawn together.

use ferrocade::frame::{self, FrameCommand};
use ferrocade::store::{self, ListId};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use pyo3::{PyTraverseError, PyVisit};

use crate::errors::IntoPyResult;
use crate::py_sprite::PySprite;

#[pyclass(name = "SpriteList", module = "ferrocade")]
pub struct PySpriteList {
    id: ListId,
    /// The member sprites' Python objects, in list order; keeps them alive while members.
    refs: Vec<Py<PySprite>>,
}

impl PySpriteList {
    pub(crate) fn list_id(&self) -> ListId {
        self.id
    }

    /// Member sprites' Python objects, in list order (slot `i` is `refs()[i]`).
    pub(crate) fn refs(&self) -> &[Py<PySprite>] {
        &self.refs
    }

    fn pairs<'py>(
        py: Python<'py>,
        pairs: &[(u32, u32)],
        a: &[Py<PySprite>],
        b: &[Py<PySprite>],
    ) -> PyResult<Bound<'py, PyList>> {
        let tuples = pairs
            .iter()
            .map(|&(i, j)| {
                PyTuple::new(
                    py,
                    [a[i as usize].clone_ref(py), b[j as usize].clone_ref(py)],
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        PyList::new(py, tuples)
    }
}

#[pymethods]
impl PySpriteList {
    #[new]
    fn new() -> Self {
        Self {
            id: store::global().create_list(),
            refs: Vec::new(),
        }
    }

    /// Add a sprite to the end of the list (drawn on top of earlier sprites).
    fn append(&mut self, sprite: &Bound<'_, PySprite>) -> PyResult<()> {
        let sprite_id = sprite.borrow().id()?;
        store::global().append(self.id, sprite_id).py()?;
        self.refs.push(sprite.clone().unbind());
        Ok(())
    }

    /// Remove a sprite, keeping the order of the others.
    fn remove(&mut self, sprite: &Bound<'_, PySprite>) -> PyResult<()> {
        let sprite_id = sprite.borrow().id()?;
        store::global().remove(self.id, sprite_id).py()?;
        let sprite_ptr = sprite.as_ptr();
        if let Some(pos) = self.refs.iter().position(|r| r.as_ptr() == sprite_ptr) {
            self.refs.remove(pos);
        }
        Ok(())
    }

    /// Move every sprite by its velocities over `delta_time` seconds (one engine call).
    fn update(&self, delta_time: f64) -> PyResult<()> {
        store::global().update_list(self.id, delta_time).py()
    }

    /// Draw every sprite, in order. Only valid inside `on_draw()`.
    fn draw(&self) -> PyResult<()> {
        frame::push(FrameCommand::DrawList(self.id), "draw").py()
    }

    /// Every touching `(sprite from this list, sprite from others)` pair once, in list order.
    /// A sprite in both lists is never paired with itself. One engine call.
    fn collisions_with<'py>(
        &self,
        py: Python<'py>,
        others: PyRef<'py, PySpriteList>,
    ) -> PyResult<Bound<'py, PyList>> {
        let pairs = store::global().list_vs_list(self.id, others.id).py()?;
        Self::pairs(py, &pairs, &self.refs, &others.refs)
    }

    /// Every touching pair within this list once, as `(earlier, later)` in list order.
    fn collisions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let pairs = store::global().within_list(self.id).py()?;
        Self::pairs(py, &pairs, &self.refs, &self.refs)
    }

    /// Bounce this list's circle sprites off each other as equal-mass, perfectly elastic circles.
    /// Returns the number of contacts resolved. One engine call.
    fn bounce_each_other(&self) -> PyResult<usize> {
        store::global().bounce_within(self.id).py()
    }

    /// Bounce this list's circle sprites off `walls` (which never move). Returns the number of
    /// contacts resolved. One engine call.
    fn bounce_off(&self, walls: PyRef<'_, PySpriteList>) -> PyResult<usize> {
        store::global().bounce_off(self.id, walls.id).py()
    }

    fn __len__(&self) -> usize {
        self.refs.len()
    }

    fn __contains__(&self, sprite: &Bound<'_, PySprite>) -> bool {
        match sprite.borrow().id() {
            Ok(sprite_id) => store::global().contains(self.id, sprite_id),
            Err(_) => false,
        }
    }

    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        // Iterate over a snapshot, so changing the list while looping is safe.
        let snapshot = PyList::new(py, self.refs.iter().map(|r| r.clone_ref(py)))?;
        Ok(snapshot.try_iter()?.into_any().unbind())
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        for sprite in &self.refs {
            visit.call(sprite)?;
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        store::global().destroy_list(self.id);
        self.refs.clear();
    }
}

impl Drop for PySpriteList {
    fn drop(&mut self) {
        // The store lock is released before `refs` drops (which may destroy sprites).
        store::global().destroy_list(self.id);
    }
}
