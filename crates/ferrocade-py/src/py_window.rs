//! `ferrocade.Window`: subclass it, override `on_draw` / `on_update`, call `run()`.

use ferrocade::color;
use ferrocade::frame::{self, FrameCommand};
use ferrocade::window::{GameCallbacks, RunError, WindowConfig, WindowState};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

use crate::errors::{IntoPyResult, to_pyerr};
use crate::py_filter::PyFilter;

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
        Ok(self.config()?.title.clone())
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

    /// Called once per displayed frame. Override to draw.
    fn on_draw(&self) {}

    /// Called at a fixed rate with the time step in seconds. Override to update the game.
    fn on_update(&self, delta_time: f64) {
        let _ = delta_time;
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

    fn check_interrupt(&mut self) -> PyResult<()> {
        self.window.py().check_signals()
    }
}
