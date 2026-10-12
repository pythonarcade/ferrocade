//! `ferrocade.Window`: subclass it, override `on_draw` / `on_update`, call `run()`.

use ferrocade::color;
use ferrocade::frame::{self, FrameCommand};
use ferrocade::input::{Key, RawKeyEvent};
use ferrocade::window::{GameCallbacks, RunError, WindowConfig, WindowState};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

use crate::errors::{IntoPyResult, to_pyerr};
use crate::py_filter::PyFilter;

/// Frame timing averaged over the last ~1 second (draw counts are from the last frame).
#[pyclass(frozen, name = "FrameStats", module = "ferrocade")]
pub struct PyFrameStats {
    /// Frames per second.
    #[pyo3(get)]
    fps: f64,
    /// Average time between frames, in milliseconds.
    #[pyo3(get)]
    frame_ms: f64,
    /// Average engine render CPU time per frame (excludes waiting for vsync), in milliseconds.
    #[pyo3(get)]
    render_cpu_ms: f64,
    #[pyo3(get)]
    draw_calls: u32,
    #[pyo3(get)]
    instances: u32,
}

#[pymethods]
impl PyFrameStats {
    fn __repr__(&self) -> String {
        format!(
            "FrameStats(fps={:.1}, frame_ms={:.2}, render_cpu_ms={:.3}, draw_calls={}, instances={})",
            self.fps, self.frame_ms, self.render_cpu_ms, self.draw_calls, self.instances
        )
    }
}

#[pyclass(subclass, name = "Window", module = "ferrocade")]
pub struct PyWindow {
    /// Set by `__init__`; `None` if a subclass forgot to call `super().__init__()`.
    config: Option<WindowConfig>,
    state: WindowState,
}

impl PyWindow {
    fn config(&self) -> PyResult<&WindowConfig> {
        self.config.as_ref().ok_or_else(|| {
            PyRuntimeError::new_err(
                "Window.__init__() was not called; call super().__init__(...) in your subclass",
            )
        })
    }
}

#[pymethods]
impl PyWindow {
    // Accept anything here so Python subclasses can pass their own arguments; the real
    // initialization happens in __init__ (which subclasses reach via super().__init__()).
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    fn new(_args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>) -> Self {
        Self {
            config: None,
            state: WindowState::new(0, 0),
        }
    }

    #[pyo3(signature = (width=800, height=600, title="Ferrocade".to_string(), *, filter, update_rate=1.0 / 60.0))]
    fn __init__(
        &mut self,
        width: i64,
        height: i64,
        title: String,
        filter: PyFilter,
        update_rate: f64,
    ) -> PyResult<()> {
        let to_size = |v: i64| u32::try_from(v).unwrap_or(0);
        let config = WindowConfig {
            width: to_size(width),
            height: to_size(height),
            title,
            update_rate,
            filter: filter.into(),
        };
        if width < 0 || height < 0 {
            return Err(to_pyerr(ferrocade::Error::InvalidArgument(format!(
                "window width and height must be > 0, got {width}x{height}"
            ))));
        }
        config.validate().py()?;
        self.state = WindowState::new(config.width, config.height);
        self.state.set_title(&config.title);
        self.config = Some(config);
        Ok(())
    }

    /// Window width in logical pixels.
    #[getter]
    fn width(&self) -> PyResult<u32> {
        self.config()?;
        Ok(self.state.size().0)
    }

    /// Window height in logical pixels.
    #[getter]
    fn height(&self) -> PyResult<u32> {
        self.config()?;
        Ok(self.state.size().1)
    }

    #[getter]
    fn title(&self) -> PyResult<String> {
        self.config()?;
        Ok(self.state.title())
    }

    /// Change the window title (applied on the next frame while running).
    #[setter]
    fn set_title(&self, title: String) -> PyResult<()> {
        self.config()?;
        self.state.set_title(&title);
        Ok(())
    }

    /// Filter for textures that don't set their own.
    #[getter]
    fn filter(&self) -> PyResult<PyFilter> {
        Ok(self.config()?.filter.into())
    }

    /// Seconds between `on_update` calls.
    #[getter]
    fn update_rate(&self) -> PyResult<f64> {
        Ok(self.config()?.update_rate)
    }

    /// Frame timing of the running window (all zeros before `run()`).
    #[getter]
    fn frame_stats(&self) -> PyFrameStats {
        let s = self.state.frame_stats();
        PyFrameStats {
            fps: s.fps,
            frame_ms: s.frame_ms,
            render_cpu_ms: s.render_cpu_ms,
            draw_calls: s.draw_calls,
            instances: s.instances,
        }
    }

    /// Show FPS and frame time in the window title (refreshed about once per second).
    #[getter]
    fn show_fps(&self) -> bool {
        self.state.show_fps()
    }

    #[setter]
    fn set_show_fps(&self, on: bool) {
        self.state.set_show_fps(on);
    }

    /// Called once per displayed frame. Override to draw.
    fn on_draw(&self) {}

    /// Called at a fixed rate with the time step in seconds. Override to update the game.
    fn on_update(&self, delta_time: f64) {
        let _ = delta_time;
    }

    /// Called once when a key goes down (auto-repeat is ignored). Override to react.
    fn on_key_press(&self, key: u16, modifiers: u8) {
        let _ = (key, modifiers);
    }

    /// Called once when a key goes up (also for held keys when the window loses focus).
    fn on_key_release(&self, key: u16, modifiers: u8) {
        let _ = (key, modifiers);
    }

    /// Whether `key` is held, as of the key events delivered so far.
    fn is_key_pressed(&self, key: i64) -> bool {
        u16::try_from(key)
            .ok()
            .and_then(Key::from_code)
            .is_some_and(|k| self.state.is_key_held(k))
    }

    /// Private (tests): queue a key event as if the platform had sent it.
    #[pyo3(signature = (key, pressed, repeat=false))]
    fn _inject_key(&self, key: i64, pressed: bool, repeat: bool) -> PyResult<()> {
        let key = u16::try_from(key)
            .ok()
            .and_then(Key::from_code)
            .ok_or_else(|| PyValueError::new_err(format!("unknown key code {key}")))?;
        self.state.push_key(RawKeyEvent {
            key,
            pressed,
            repeat,
        });
        Ok(())
    }

    /// Private (tests): act as if the window lost focus.
    fn _inject_focus_lost(&self) {
        self.state.release_all_keys();
    }

    /// Fill the window with a color. Only valid inside `on_draw()`.
    #[pyo3(signature = (color=None))]
    fn clear(&self, color: Option<Vec<i64>>) -> PyResult<()> {
        let rgba = match color {
            Some(components) => color::parse_rgba(&components).py()?,
            None => color::DEFAULT_BACKGROUND,
        };
        frame::push(FrameCommand::Clear(color::to_linear_f32(rgba)), "clear").py()
    }

    /// Open the window and run the game until it is closed.
    fn run(slf: &Bound<'_, Self>) -> PyResult<()> {
        let py = slf.py();
        let threading = py.import("threading")?;
        let current = threading.call_method0("current_thread")?;
        let main = threading.call_method0("main_thread")?;
        if !current.is(&main) {
            return Err(PyRuntimeError::new_err(
                "Window.run() must be called from the main thread",
            ));
        }

        // Copy what the loop needs so no borrow of `self` is held while Python callbacks run.
        let (config, state) = {
            let this = slf.borrow();
            (this.config()?.clone(), this.state.clone())
        };
        let mut callbacks = PyCallbacks {
            window: slf.clone().into_any(),
        };
        match ferrocade::platform::run(&config, state, &mut callbacks) {
            Ok(()) => Ok(()),
            Err(RunError::Callback(err)) => Err(err),
            Err(RunError::Engine(err)) => Err(to_pyerr(err)),
        }
    }
}

/// Dispatches the game loop's callbacks to the Python object, so subclass overrides run.
struct PyCallbacks<'py> {
    window: Bound<'py, PyAny>,
}

impl GameCallbacks for PyCallbacks<'_> {
    type Error = PyErr;

    fn update(&mut self, delta_time: f64) -> PyResult<()> {
        self.window.call_method1("on_update", (delta_time,))?;
        Ok(())
    }

    fn draw(&mut self) -> PyResult<()> {
        self.window.call_method0("on_draw")?;
        Ok(())
    }

    fn key_press(&mut self, key: Key, modifiers: u8) -> PyResult<()> {
        self.window
            .call_method1("on_key_press", (key.code(), modifiers))?;
        Ok(())
    }

    fn key_release(&mut self, key: Key, modifiers: u8) -> PyResult<()> {
        self.window
            .call_method1("on_key_release", (key.code(), modifiers))?;
        Ok(())
    }

    fn check_interrupt(&mut self) -> PyResult<()> {
        self.window.py().check_signals()
    }
}
