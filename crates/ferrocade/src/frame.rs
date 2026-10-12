//! Per-frame draw queue.
//!
//! While `on_draw` runs, `Window.clear()` and `SpriteList.draw()` only record commands here.
//! After `on_draw` returns, the platform layer hands the commands to the renderer. This keeps
//! the renderer entirely inside Rust, with no re-entrant access from Python mid-frame.

use std::cell::RefCell;

use crate::store::ListId;
use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FrameCommand {
    /// Clear to this linear-space RGBA color.
    Clear([f64; 4]),
    DrawList(ListId),
}

#[derive(Default)]
struct FrameQueue {
    active: bool,
    commands: Vec<FrameCommand>,
}

thread_local! {
    static QUEUE: RefCell<FrameQueue> = RefCell::new(FrameQueue::default());
}

/// Start recording a frame.
pub fn begin() {
    QUEUE.with_borrow_mut(|q| {
        q.active = true;
        q.commands.clear();
    });
}

/// Stop recording and return the frame's commands in call order.
pub fn end() -> Vec<FrameCommand> {
    QUEUE.with_borrow_mut(|q| {
        q.active = false;
        std::mem::take(&mut q.commands)
    })
}

/// Record a command. `op` names the user-facing call for the error message.
pub fn push(cmd: FrameCommand, op: &'static str) -> Result<()> {
    QUEUE.with_borrow_mut(|q| {
        if !q.active {
            return Err(Error::NotInFrame { op });
        }
        q.commands.push(cmd);
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_outside_frame_fails() {
        let err = push(FrameCommand::Clear([0.0; 4]), "clear").unwrap_err();
        assert_eq!(
            err.to_string(),
            "clear() can only be called inside on_draw()"
        );
    }

    #[test]
    fn records_commands_in_order() {
        begin();
        push(FrameCommand::Clear([1.0; 4]), "clear").unwrap();
        push(FrameCommand::Clear([0.5; 4]), "clear").unwrap();
        let cmds = end();
        assert_eq!(
            cmds,
            vec![FrameCommand::Clear([1.0; 4]), FrameCommand::Clear([0.5; 4])]
        );
        assert!(push(FrameCommand::Clear([0.0; 4]), "clear").is_err());
    }
}
