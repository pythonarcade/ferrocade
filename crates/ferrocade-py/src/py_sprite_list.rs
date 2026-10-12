//! `ferrocade.SpriteList`: an ordered batch of sprites, drawn together.

use ferrocade::frame::{self, FrameCommand};
use ferrocade::store::{self, ListId};
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::{PyTraverseError, PyVisit};

use crate::errors::IntoPyResult;
use crate::py_sprite::PySprite;

#[pyclass(name = "SpriteList", module = "ferrocade")]
pub struct PySpriteList {
    id: ListId,
    /// The member sprites' Python objects, in list order; keeps them alive while members.
    refs: Vec<Py<PySprite>>,
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

    /// Draw every sprite, in order. Only valid inside `on_draw()`.
    fn draw(&self) -> PyResult<()> {
        frame::push(FrameCommand::DrawList(self.id), "draw").py()
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
