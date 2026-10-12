//! Textures: decoded images shared between sprites, and the texture filter choice.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use crate::{Error, Result, assets};

/// How a texture is sampled when drawn.
///
/// There is deliberately no `Default`: the right choice depends on the game's art style, so the
/// window requires an explicit filter and textures may override it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Filter {
    /// Crisp pixels with no smoothing (pixel art).
    Nearest,
    /// Smooth interpolation (photos, scaled or rotated art).
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureId(u64);

/// A decoded image, shared by every sprite and texture handle that uses the same file.
///
/// This is CPU data only; each renderer uploads it to the GPU on first use.
#[derive(Debug)]
pub struct TextureData {
    pub id: TextureId,
    /// Canonical absolute path (the cache key).
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    /// RGBA8, straight alpha, top row first.
    pub pixels: Arc<[u8]>,
}

/// A texture as sprites use it: shared image data plus an optional filter override.
#[derive(Clone, Debug)]
pub struct TextureRef {
    pub data: Arc<TextureData>,
    /// `None` means "use the window's filter".
    pub filter: Option<Filter>,
}

impl TextureRef {
    pub fn effective_filter(&self, window_filter: Filter) -> Filter {
        self.filter.unwrap_or(window_filter)
    }
}

impl PartialEq for TextureRef {
    fn eq(&self, other: &Self) -> bool {
        self.data.id == other.data.id && self.filter == other.filter
    }
}

impl Eq for TextureRef {}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static DISK_READS: AtomicU64 = AtomicU64::new(0);
// Strong references: textures are never unloaded in Phase 1.
static CACHE: LazyLock<Mutex<HashMap<PathBuf, Arc<TextureData>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Load an image, reading and decoding each distinct file only once per process.
pub fn load(path: &Path) -> Result<Arc<TextureData>> {
    let canonical = assets::canonicalize(path)?;
    let mut cache = CACHE.lock().unwrap();
    if let Some(data) = cache.get(&canonical) {
        return Ok(data.clone());
    }
    let bytes = assets::read_bytes(&canonical)?;
    DISK_READS.fetch_add(1, Ordering::Relaxed);
    let data = decode(&canonical, &bytes)?;
    cache.insert(canonical, data.clone());
    Ok(data)
}

/// Load an image from bytes already in memory (e.g. fetched by a browser), cached by `path`.
///
/// `path` is used as given (not canonicalized), since there may be no filesystem to resolve it.
pub fn load_from_bytes(path: &Path, bytes: &[u8]) -> Result<Arc<TextureData>> {
    let mut cache = CACHE.lock().unwrap();
    if let Some(data) = cache.get(path) {
        return Ok(data.clone());
    }
    let data = decode(path, bytes)?;
    cache.insert(path.to_path_buf(), data.clone());
    Ok(data)
}

fn decode(path: &Path, bytes: &[u8]) -> Result<Arc<TextureData>> {
    let image = image::load_from_memory(bytes).map_err(|e| Error::Decode {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;
    let rgba = image.into_rgba8();
    Ok(Arc::new(TextureData {
        id: TextureId(NEXT_ID.fetch_add(1, Ordering::Relaxed)),
        path: path.to_path_buf(),
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw().into(),
    }))
}

/// Number of image files read from disk so far (for verifying the cache).
pub fn disk_reads() -> u64 {
    DISK_READS.load(Ordering::Relaxed)
}

/// Cache key of the built-in shape texture (never a real file path).
pub const BUILTIN_SHAPES_KEY: &str = "<builtin:shapes>";
/// Texture region for rectangles: inset into the white half, so filtering never samples edges.
pub const RECT_UV: [f32; 4] = [0.125, 0.25, 0.375, 0.75];
/// Texture region for circles: the disc in the right half.
pub const CIRCLE_UV: [f32; 4] = [0.5, 0.0, 1.0, 1.0];

/// The built-in shape texture used by rectangle and circle sprites, created on first use.
///
/// 1024×512: the left half is opaque white (rectangles), the right half a white disc with an
/// anti-aliased edge (circles). Both shapes share it, so any mix of them batches together.
pub fn builtin_shapes() -> Arc<TextureData> {
    let key = PathBuf::from(BUILTIN_SHAPES_KEY);
    let mut cache = CACHE.lock().unwrap();
    if let Some(data) = cache.get(&key) {
        return data.clone();
    }
    const W: u32 = 1024;
    const H: u32 = 512;
    let (cx, cy, radius) = (768.0f32, 256.0f32, 254.0f32);
    let mut pixels = vec![0u8; (W * H * 4) as usize];
    for y in 0..H {
        for x in 0..W {
            let alpha = if x < W / 2 {
                1.0
            } else {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                (radius + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0)
            };
            let i = ((y * W + x) * 4) as usize;
            pixels[i..i + 4].copy_from_slice(&[255, 255, 255, (alpha * 255.0).round() as u8]);
        }
    }
    let data = Arc::new(TextureData {
        id: TextureId(NEXT_ID.fetch_add(1, Ordering::Relaxed)),
        path: key.clone(),
        width: W,
        height: H,
        pixels: pixels.into(),
    });
    cache.insert(key, data.clone());
    data
}
