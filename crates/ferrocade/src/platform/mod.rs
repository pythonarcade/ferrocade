//! Platform layer: the only code that knows about windowing systems and event loops.

#[cfg(not(target_arch = "wasm32"))]
mod desktop;

use crate::window::{GameCallbacks, RunError, WindowConfig, WindowState};

/// Open a window and run the game loop until it closes or a callback fails.
pub fn run<C: GameCallbacks>(
    config: &WindowConfig,
    state: WindowState,
    callbacks: &mut C,
) -> Result<(), RunError<C::Error>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        desktop::run(config, state, callbacks)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (config, state, callbacks);
        Err(RunError::Engine(crate::Error::Unsupported {
            path: Default::default(),
            reason: "running a window in the browser is not implemented yet (Phase 1b)".into(),
        }))
    }
}
