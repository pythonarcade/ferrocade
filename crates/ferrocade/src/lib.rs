//! Ferrocade engine core.
//!
//! This crate has no Python dependency; the `ferrocade-py` crate wraps it for Python.

pub mod assets;
pub mod collision;
pub mod color;
pub mod error;
pub mod frame;
pub mod input;
pub mod instance;
#[cfg(feature = "render")]
pub mod platform;
#[cfg(feature = "render")]
pub mod render;
pub mod store;
pub mod texture;
pub mod timestep;
pub mod window;

pub use error::{Error, Result};
