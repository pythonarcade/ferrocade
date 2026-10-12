//! Asset loader: the only place the engine touches files.
//!
//! On desktop this reads from the filesystem. Browsers have no filesystem, so the web build
//! will fetch or preload assets instead (Phase 1b); engine code must always go through here.

use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Resolve `path` (relative to the current working directory) to a canonical absolute path.
#[cfg(not(target_arch = "wasm32"))]
pub fn canonicalize(path: &Path) -> Result<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(p) => Ok(strip_verbatim_prefix(p)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::NotFound {
            path: path.to_path_buf(),
        }),
        Err(source) => Err(Error::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Read the whole file at `path`.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            Error::NotFound {
                path: path.to_path_buf(),
            }
        } else {
            Error::Io {
                path: path.to_path_buf(),
                source,
            }
        }
    })
}

#[cfg(target_arch = "wasm32")]
pub fn canonicalize(path: &Path) -> Result<PathBuf> {
    Ok(path.to_path_buf())
}

#[cfg(target_arch = "wasm32")]
pub fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    Err(Error::Unsupported {
        path: path.to_path_buf(),
        reason: "file loading is not implemented for the web yet".into(),
    })
}

/// Windows `canonicalize` returns `\\?\C:\...` paths; show users the familiar `C:\...` form.
#[cfg(not(target_arch = "wasm32"))]
fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        // Only plain drive paths; leave UNC (`\\?\UNC\...`) paths untouched.
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => path,
    }
}
