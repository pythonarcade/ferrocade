//! Window configuration, shared window state, and the callback interface for the game loop.

use std::sync::{Arc, Mutex};

use crate::input::{Key, KeyEvent, Keyboard, RawKeyEvent};
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
pub struct WindowState(Arc<Mutex<Inner>>);

#[derive(Debug, Default)]
struct Inner {
    width: u32,
    height: u32,
    stats: RollingStats,
    show_fps: bool,
    keyboard: Keyboard,
}

/// Frame statistics averaged over the last ~1 second (draw counts are from the last frame).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameSnapshot {
    pub fps: f64,
    pub frame_ms: f64,
    pub render_cpu_ms: f64,
    pub draw_calls: u32,
    pub instances: u32,
}

/// Accumulates per-frame timings and rolls them into a snapshot about once per second.
#[derive(Debug, Default)]
struct RollingStats {
    frames: u32,
    seconds: f64,
    render_ms: f64,
    snapshot: FrameSnapshot,
    rolled_once: bool,
}

const STATS_WINDOW_SECONDS: f64 = 1.0;

impl RollingStats {
    /// Returns true when a new ~1 s window was completed.
    fn record(
        &mut self,
        interval: f64,
        render_cpu_ms: f64,
        draw_calls: u32,
        instances: u32,
    ) -> bool {
        self.frames += 1;
        self.seconds += interval;
        self.render_ms += render_cpu_ms;
        let averaged = FrameSnapshot {
            fps: if self.seconds > 0.0 {
                self.frames as f64 / self.seconds
            } else {
                0.0
            },
            frame_ms: self.seconds * 1000.0 / self.frames as f64,
            render_cpu_ms: self.render_ms / self.frames as f64,
            draw_calls,
            instances,
        };
        let rolled = self.seconds >= STATS_WINDOW_SECONDS;
        if rolled || !self.rolled_once {
            // Until the first full window, report the values so far.
            self.snapshot = averaged;
        } else {
            self.snapshot.draw_calls = draw_calls;
            self.snapshot.instances = instances;
        }
        if rolled {
            self.rolled_once = true;
            (self.frames, self.seconds, self.render_ms) = (0, 0.0, 0.0);
        }
        rolled
    }
}

/// Window title with the FPS readout appended.
pub fn fps_title(title: &str, stats: &FrameSnapshot) -> String {
    format!("{title} — {:.0} FPS, {:.2} ms", stats.fps, stats.frame_ms)
}

impl WindowState {
    pub fn new(width: u32, height: u32) -> Self {
        Self(Arc::new(Mutex::new(Inner {
            width,
            height,
            ..Default::default()
        })))
    }

    pub fn size(&self) -> (u32, u32) {
        let s = self.0.lock().unwrap();
        (s.width, s.height)
    }

    /// Record a new logical size. A zero size (minimized) keeps the last non-zero size.
    pub fn set_size(&self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let mut s = self.0.lock().unwrap();
            (s.width, s.height) = (width, height);
        }
    }

    /// Record one frame's timing. Returns the new snapshot when a ~1 s window completes.
    pub fn record_frame(
        &self,
        interval: f64,
        render_cpu_ms: f64,
        draw_calls: u32,
        instances: u32,
    ) -> Option<FrameSnapshot> {
        let mut s = self.0.lock().unwrap();
        s.stats
            .record(interval, render_cpu_ms, draw_calls, instances)
            .then_some(s.stats.snapshot)
    }

    pub fn frame_stats(&self) -> FrameSnapshot {
        self.0.lock().unwrap().stats.snapshot
    }

    /// Start a new run with fresh statistics.
    pub fn reset_stats(&self) {
        self.0.lock().unwrap().stats = RollingStats::default();
    }

    /// Start a new run with no keys held or queued.
    pub fn reset_input(&self) {
        self.0.lock().unwrap().keyboard.reset();
    }

    /// Queue a key event from the platform (or a test).
    pub fn push_key(&self, event: RawKeyEvent) {
        self.0.lock().unwrap().keyboard.push(event);
    }

    /// Release every held key (the window lost focus).
    pub fn release_all_keys(&self) {
        self.0.lock().unwrap().keyboard.release_all();
    }

    /// Apply queued key events; returns the ones to deliver to the game, in order.
    pub fn drain_keys(&self) -> Vec<KeyEvent> {
        self.0.lock().unwrap().keyboard.drain()
    }

    pub fn is_key_held(&self, key: Key) -> bool {
        self.0.lock().unwrap().keyboard.is_held(key)
    }

    pub fn show_fps(&self) -> bool {
        self.0.lock().unwrap().show_fps
    }

    pub fn set_show_fps(&self, on: bool) {
        self.0.lock().unwrap().show_fps = on;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_roll_about_once_per_second() {
        let mut s = RollingStats::default();
        let mut rolls = 0;
        for _ in 0..120 {
            if s.record(1.0 / 60.0, 2.0, 3, 30) {
                rolls += 1;
            }
        }
        assert_eq!(rolls, 2);
        assert!((s.snapshot.fps - 60.0).abs() < 0.5, "{:?}", s.snapshot);
        assert!((s.snapshot.frame_ms - 16.667).abs() < 0.1);
        assert!((s.snapshot.render_cpu_ms - 2.0).abs() < 1e-9);
        assert_eq!((s.snapshot.draw_calls, s.snapshot.instances), (3, 30));
    }

    #[test]
    fn stats_report_values_before_first_window() {
        let mut s = RollingStats::default();
        s.record(0.02, 1.0, 1, 5);
        assert!((s.snapshot.fps - 50.0).abs() < 1e-9);
    }

    #[test]
    fn fps_title_format() {
        let snap = FrameSnapshot {
            fps: 59.6,
            frame_ms: 16.6667,
            ..Default::default()
        };
        assert_eq!(fps_title("Game", &snap), "Game — 60 FPS, 16.67 ms");
    }
}

/// The game's callbacks, implemented by the bindings (which call into Python).
pub trait GameCallbacks {
    type Error;
    fn update(&mut self, delta_time: f64) -> std::result::Result<(), Self::Error>;
    fn draw(&mut self) -> std::result::Result<(), Self::Error>;
    /// A key went down (`modifiers`: `input::MOD_*` flags held after the event).
    fn key_press(&mut self, key: Key, modifiers: u8) -> std::result::Result<(), Self::Error> {
        let _ = (key, modifiers);
        Ok(())
    }
    /// A key went up.
    fn key_release(&mut self, key: Key, modifiers: u8) -> std::result::Result<(), Self::Error> {
        let _ = (key, modifiers);
        Ok(())
    }
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
