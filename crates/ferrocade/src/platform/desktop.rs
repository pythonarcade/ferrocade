//! Desktop (Windows/macOS/Linux) game loop on winit.

use std::cell::RefCell;
use std::sync::Arc;

use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::{Window, WindowId};

use crate::render::Renderer;
use crate::timestep::FixedTimestep;
use crate::window::{GameCallbacks, RunError, WindowConfig, WindowState};
use crate::{Error, frame};

thread_local! {
    // winit allows one event loop per process, so it is created once and reused by every
    // run() call (run_app_on_demand lets it run again after a window closes).
    static EVENT_LOOP: RefCell<Option<EventLoop<()>>> = const { RefCell::new(None) };
}

pub fn run<C: GameCallbacks>(
    config: &WindowConfig,
    state: WindowState,
    callbacks: &mut C,
) -> Result<(), RunError<C::Error>> {
    EVENT_LOOP.with(|cell| {
        let mut slot = cell
            .try_borrow_mut()
            .map_err(|_| Error::EventLoop("run() is already running".into()))?;
        if slot.is_none() {
            *slot = Some(EventLoop::new().map_err(|e| Error::EventLoop(e.to_string()))?);
        }
        let event_loop = slot.as_mut().expect("event loop was just created");
        event_loop.set_control_flow(ControlFlow::Wait);

        let mut app = App {
            config,
            state,
            callbacks,
            window: None,
            renderer: None,
            timestep: FixedTimestep::new(config.update_rate),
            last_frame: None,
            error: None,
        };
        event_loop
            .run_app_on_demand(&mut app)
            .map_err(|e| Error::EventLoop(e.to_string()))?;
        // Close the window even if the loop stopped without a CloseRequested event.
        app.renderer = None;
        app.window = None;
        match app.error.take() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    })
}

struct App<'a, C: GameCallbacks> {
    config: &'a WindowConfig,
    state: WindowState,
    callbacks: &'a mut C,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    timestep: FixedTimestep,
    last_frame: Option<Instant>,
    error: Option<RunError<C::Error>>,
}

impl<C: GameCallbacks> App<'_, C> {
    /// Stop the loop, remembering why. The window is dropped before the loop exits.
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: RunError<C::Error>) {
        if self.error.is_none() {
            self.error = Some(error);
        }
        self.shutdown(event_loop);
    }

    fn shutdown(&mut self, event_loop: &ActiveEventLoop) {
        self.renderer = None;
        self.window = None;
        event_loop.exit();
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> crate::Result<()> {
        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|e| Error::EventLoop(format!("could not create window: {e}")))?,
        );
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(
            Box::new(event_loop.owned_display_handle()),
        ));
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| Error::Gpu(format!("could not create window surface: {e}")))?;
        let size = window.inner_size();
        let renderer = pollster::block_on(Renderer::with_surface(
            &instance,
            surface,
            size.width,
            size.height,
        ))?;
        self.renderer = Some(renderer);
        window.request_redraw();
        self.window = Some(window);
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let elapsed = self
            .last_frame
            .map_or(0.0, |last| (now - last).as_secs_f64());
        self.last_frame = Some(now);

        let step = self.timestep.step();
        for _ in 0..self.timestep.advance(elapsed) {
            if let Err(e) = self.callbacks.update(step) {
                return self.fail(event_loop, RunError::Callback(e));
            }
        }

        frame::begin();
        let drawn = self.callbacks.draw();
        let commands = frame::end();
        if let Err(e) = drawn {
            return self.fail(event_loop, RunError::Callback(e));
        }

        if let Some(renderer) = &mut self.renderer
            && let Err(e) = renderer.render_frame(&commands, self.state.size(), self.config.filter)
        {
            return self.fail(event_loop, RunError::Engine(e));
        }

        if let Err(e) = self.callbacks.check_interrupt() {
            return self.fail(event_loop, RunError::Callback(e));
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl<C: GameCallbacks> ApplicationHandler for App<'_, C> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(e) = self.create_window(event_loop)
        {
            self.fail(event_loop, RunError::Engine(e));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.shutdown(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                if let Some(window) = &self.window {
                    let logical = size.to_logical::<f64>(window.scale_factor());
                    self.state
                        .set_size(logical.width.round() as u32, logical.height.round() as u32);
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }
}
