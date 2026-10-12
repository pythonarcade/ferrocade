//! Window configuration, shared window state, and the callback interface for the game loop.

use std::sync::{Arc, Mutex};

use crate::texture::Filter;
use crate::{Error, Result};

#[derive(Clone, Debug)]
pub struct WindowConfig {
    /// Logical pixels.
    pub width: u32,
    pub height: u32,
    pub title: String,
    /// Seconds per `on_update` call.
    pub update_rate: f64,
    /// Filter for textures that don't override it. Required: there is no default.
    pub filter: Filter,
}

impl WindowConfig {
    pub fn validate(&self) -> Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(Error::InvalidArgument(format!(
                "window width and height must be > 0, got {}x{}",
                self.width, self.height
            )));
        }
        if !(self.update_rate.is_finite() && self.update_rate > 0.0) {
            return Err(Error::InvalidArgument(format!(
                "update_rate must be > 0, got {}",
                self.update_rate
            )));
        }
        Ok(())
    }
}

/// Window state that changes while the window runs, readable from the bindings.
#[derive(Clone, Debug)]
pub struct WindowState(Arc<Mutex<LogicalSize>>);

#[derive(Clone, Copy, Debug)]
struct LogicalSize {
    width: u32,
    height: u32,
}

impl WindowState {
    pub fn new(width: u32, height: u32) -> Self {
        Self(Arc::new(Mutex::new(LogicalSize { width, height })))
    }

    pub fn size(&self) -> (u32, u32) {
        let s = self.0.lock().unwrap();
        (s.width, s.height)
    }

    /// Record a new logical size. A zero size (minimized) keeps the last non-zero size.
    pub fn set_size(&self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            *self.0.lock().unwrap() = LogicalSize { width, height };
        }
    }
}

/// The game's callbacks, implemented by the bindings (which call into Python).
pub trait GameCallbacks {
    type Error;
    fn update(&mut self, delta_time: f64) -> std::result::Result<(), Self::Error>;
    fn draw(&mut self) -> std::result::Result<(), Self::Error>;
    /// Called once per frame so the host can surface interrupts (e.g. Ctrl+C).
    fn check_interrupt(&mut self) -> std::result::Result<(), Self::Error>;
}

/// Why `run` stopped early: a callback failed, or the engine did.
#[derive(Debug)]
pub enum RunError<E> {
    Callback(E),
    Engine(Error),
}

impl<E> From<Error> for RunError<E> {
    fn from(e: Error) -> Self {
        RunError::Engine(e)
    }
}
