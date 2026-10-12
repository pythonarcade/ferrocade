//! SPIKE: browser bindings for Ferrocade (Phase 1b, approach B).
//!
//! Exposes the engine core to JavaScript via wasm-bindgen; a pure-Python shim running in
//! Pyodide calls this API. Only compiled for `wasm32-unknown-unknown`, so desktop workspace
//! builds are unaffected.

#![cfg(target_arch = "wasm32")]
