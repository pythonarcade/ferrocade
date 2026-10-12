//! Desktop (Windows/macOS/Linux) game loop on winit.

use std::cell::RefCell;
use std::sync::Arc;

use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::{Window, WindowId};

use crate::input::{Key, RawKeyEvent};
use crate::render::Renderer;
use crate::timestep::FixedTimestep;
use crate::window::{GameCallbacks, RunError, WindowConfig, WindowState, fps_title};
use crate::{Error, frame, store};

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

        state.reset_stats();
        state.reset_input();
        // The window is created with the current title.
        state.take_title_change();
        let mut app = App {
            config,
            state,
            callbacks,
            window: None,
            renderer: None,
            timestep: FixedTimestep::new(config.update_rate),
            last_frame: None,
            fps_in_title: false,
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
    /// Whether the title currently shows the FPS readout (so it can be restored once).
    fps_in_title: bool,
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
            .with_title(self.state.title())
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|e| Error::EventLoop(format!("could not create window: {e}")))?,
        );
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(
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

        // Key events that arrived since the last frame, delivered before this frame's updates.
        for event in self.state.drain_keys() {
            let result = if event.pressed {
                self.callbacks.key_press(event.key, event.modifiers)
            } else {
                self.callbacks.key_release(event.key, event.modifiers)
            };
            if let Err(e) = result {
                return self.fail(event_loop, RunError::Callback(e));
            }
        }

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
            && let Err(e) = renderer.render_frame(
                &commands,
                &mut store::global(),
                self.state.size(),
                self.config.filter,
            )
        {
            return self.fail(event_loop, RunError::Engine(e));
        }

        if let Some(renderer) = &self.renderer {
            let stats = renderer.last_frame_stats();
            let rolled = self.state.record_frame(
                elapsed,
                stats.render_cpu_ms,
                stats.draw_calls,
                stats.instances,
            );
            if let Some(window) = &self.window {
                let show = self.state.show_fps();
                let changed = self.state.take_title_change().is_some();
                let title = self.state.title();
                if show && (rolled.is_some() || changed) {
                    let snapshot = rolled.unwrap_or_else(|| self.state.frame_stats());
                    window.set_title(&fps_title(&title, &snapshot));
                    self.fps_in_title = true;
                } else if !show && (self.fps_in_title || changed) {
                    window.set_title(&title);
                    self.fps_in_title = false;
                }
            }
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
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key
                    && let Some(key) = map_key(code)
                {
                    self.state.push_key(RawKeyEvent {
                        key,
                        pressed: event.state.is_pressed(),
                        repeat: event.repeat,
                    });
                }
            }
            WindowEvent::Focused(false) => self.state.release_all_keys(),
            _ => {}
        }
    }
}

/// winit physical key codes for every engine key. winit names keys after the W3C UI Events
/// `code` values, except that "Meta" is called "Super".
const KEY_MAP: &[(KeyCode, Key)] = &[
    (KeyCode::KeyA, Key::A),
    (KeyCode::KeyB, Key::B),
    (KeyCode::KeyC, Key::C),
    (KeyCode::KeyD, Key::D),
    (KeyCode::KeyE, Key::E),
    (KeyCode::KeyF, Key::F),
    (KeyCode::KeyG, Key::G),
    (KeyCode::KeyH, Key::H),
    (KeyCode::KeyI, Key::I),
    (KeyCode::KeyJ, Key::J),
    (KeyCode::KeyK, Key::K),
    (KeyCode::KeyL, Key::L),
    (KeyCode::KeyM, Key::M),
    (KeyCode::KeyN, Key::N),
    (KeyCode::KeyO, Key::O),
    (KeyCode::KeyP, Key::P),
    (KeyCode::KeyQ, Key::Q),
    (KeyCode::KeyR, Key::R),
    (KeyCode::KeyS, Key::S),
    (KeyCode::KeyT, Key::T),
    (KeyCode::KeyU, Key::U),
    (KeyCode::KeyV, Key::V),
    (KeyCode::KeyW, Key::W),
    (KeyCode::KeyX, Key::X),
    (KeyCode::KeyY, Key::Y),
    (KeyCode::KeyZ, Key::Z),
    (KeyCode::Digit0, Key::Digit0),
    (KeyCode::Digit1, Key::Digit1),
    (KeyCode::Digit2, Key::Digit2),
    (KeyCode::Digit3, Key::Digit3),
    (KeyCode::Digit4, Key::Digit4),
    (KeyCode::Digit5, Key::Digit5),
    (KeyCode::Digit6, Key::Digit6),
    (KeyCode::Digit7, Key::Digit7),
    (KeyCode::Digit8, Key::Digit8),
    (KeyCode::Digit9, Key::Digit9),
    (KeyCode::F1, Key::F1),
    (KeyCode::F2, Key::F2),
    (KeyCode::F3, Key::F3),
    (KeyCode::F4, Key::F4),
    (KeyCode::F5, Key::F5),
    (KeyCode::F6, Key::F6),
    (KeyCode::F7, Key::F7),
    (KeyCode::F8, Key::F8),
    (KeyCode::F9, Key::F9),
    (KeyCode::F10, Key::F10),
    (KeyCode::F11, Key::F11),
    (KeyCode::F12, Key::F12),
    (KeyCode::ArrowLeft, Key::Left),
    (KeyCode::ArrowRight, Key::Right),
    (KeyCode::ArrowUp, Key::Up),
    (KeyCode::ArrowDown, Key::Down),
    (KeyCode::Space, Key::Space),
    (KeyCode::Enter, Key::Enter),
    (KeyCode::Escape, Key::Escape),
    (KeyCode::Tab, Key::Tab),
    (KeyCode::Backspace, Key::Backspace),
    (KeyCode::Insert, Key::Insert),
    (KeyCode::Delete, Key::Delete),
    (KeyCode::Home, Key::Home),
    (KeyCode::End, Key::End),
    (KeyCode::PageUp, Key::PageUp),
    (KeyCode::PageDown, Key::PageDown),
    (KeyCode::CapsLock, Key::CapsLock),
    (KeyCode::ShiftLeft, Key::LShift),
    (KeyCode::ShiftRight, Key::RShift),
    (KeyCode::ControlLeft, Key::LCtrl),
    (KeyCode::ControlRight, Key::RCtrl),
    (KeyCode::AltLeft, Key::LAlt),
    (KeyCode::AltRight, Key::RAlt),
    (KeyCode::SuperLeft, Key::LSystem),
    (KeyCode::SuperRight, Key::RSystem),
    (KeyCode::Numpad0, Key::Num0),
    (KeyCode::Numpad1, Key::Num1),
    (KeyCode::Numpad2, Key::Num2),
    (KeyCode::Numpad3, Key::Num3),
    (KeyCode::Numpad4, Key::Num4),
    (KeyCode::Numpad5, Key::Num5),
    (KeyCode::Numpad6, Key::Num6),
    (KeyCode::Numpad7, Key::Num7),
    (KeyCode::Numpad8, Key::Num8),
    (KeyCode::Numpad9, Key::Num9),
    (KeyCode::NumpadAdd, Key::NumAdd),
    (KeyCode::NumpadSubtract, Key::NumSubtract),
    (KeyCode::NumpadMultiply, Key::NumMultiply),
    (KeyCode::NumpadDivide, Key::NumDivide),
    (KeyCode::NumpadDecimal, Key::NumDecimal),
    (KeyCode::NumpadEnter, Key::NumEnter),
    (KeyCode::NumLock, Key::NumLock),
    (KeyCode::Minus, Key::Minus),
    (KeyCode::Equal, Key::Equal),
    (KeyCode::BracketLeft, Key::BracketLeft),
    (KeyCode::BracketRight, Key::BracketRight),
    (KeyCode::Backslash, Key::Backslash),
    (KeyCode::Semicolon, Key::Semicolon),
    (KeyCode::Quote, Key::Apostrophe),
    (KeyCode::Backquote, Key::Grave),
    (KeyCode::Comma, Key::Comma),
    (KeyCode::Period, Key::Period),
    (KeyCode::Slash, Key::Slash),
];

fn map_key(code: KeyCode) -> Option<Key> {
    KEY_MAP.iter().find(|(c, _)| *c == code).map(|e| e.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::KEYS;

    #[test]
    fn every_key_has_a_winit_code_with_the_w3c_name() {
        assert_eq!(KEY_MAP.len(), KEYS.len());
        for (key, w3c, _) in KEYS {
            let (code, _) = KEY_MAP
                .iter()
                .find(|(_, k)| k == key)
                .unwrap_or_else(|| panic!("{key:?} has no winit mapping"));
            let expected = w3c.replace("Meta", "Super");
            assert_eq!(format!("{code:?}"), expected);
            assert_eq!(map_key(*code), Some(*key));
        }
    }
}
