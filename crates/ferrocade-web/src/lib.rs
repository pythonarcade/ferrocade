//! SPIKE: browser bindings for Ferrocade (Phase 1b, approach B).
//!
//! Exposes the engine core to JavaScript via wasm-bindgen; a pure-Python shim running in
//! Pyodide calls this API. Only compiled for `wasm32-unknown-unknown`, so desktop workspace
//! builds are unaffected.
//!
//! IDs cross the boundary as `u32` handles into tables here (slotmap keys are 64-bit, and JS
//! numbers are only exact to 2^53).

#![cfg(target_arch = "wasm32")]

use std::path::Path;

use ferrocade::color;
use ferrocade::frame::{self, FrameCommand};
use ferrocade::render::Renderer;
use ferrocade::store::{self, ListId, SpriteId};
use ferrocade::texture::{self, Filter, TextureRef};
use ferrocade::timestep::FixedTimestep;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

fn js_err(e: ferrocade::Error) -> JsError {
    JsError::new(&e.to_string())
}

fn parse_filter(name: &str) -> Result<Option<Filter>, JsError> {
    match name {
        "" => Ok(None),
        "nearest" => Ok(Some(Filter::Nearest)),
        "linear" => Ok(Some(Filter::Linear)),
        other => Err(JsError::new(&format!("unknown filter {other:?}"))),
    }
}

#[wasm_bindgen]
pub struct Engine {
    canvas: HtmlCanvasElement,
    renderer: Renderer,
    logical_size: (u32, u32),
    filter: Filter,
    timestep: FixedTimestep,
    textures: Vec<TextureRef>,
    sprites: Vec<Option<SpriteId>>,
    lists: Vec<Option<ListId>>,
}

impl Engine {
    fn sprite(&self, handle: u32) -> Result<SpriteId, JsError> {
        self.sprites
            .get(handle as usize)
            .copied()
            .flatten()
            .ok_or_else(|| JsError::new("sprite no longer exists"))
    }

    fn list(&self, handle: u32) -> Result<ListId, JsError> {
        self.lists
            .get(handle as usize)
            .copied()
            .flatten()
            .ok_or_else(|| JsError::new("sprite list no longer exists"))
    }

    fn texture(&self, handle: u32) -> Result<&TextureRef, JsError> {
        self.textures
            .get(handle as usize)
            .ok_or_else(|| JsError::new("unknown texture"))
    }

    fn physical_size(&self) -> (u32, u32) {
        let ratio = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
        let (w, h) = self.logical_size;
        (
            ((w as f64) * ratio).round() as u32,
            ((h as f64) * ratio).round() as u32,
        )
    }
}

#[wasm_bindgen]
impl Engine {
    /// Initialize the GPU on `canvas`. `backend` is "auto" (WebGPU, else WebGL2) or "webgl2".
    pub async fn create(canvas: HtmlCanvasElement, backend: String) -> Result<Engine, JsError> {
        console_error_panic_hook::set_once();
        let instance = match backend.as_str() {
            "auto" => {
                wgpu::util::new_instance_with_webgpu_detection(
                    wgpu::InstanceDescriptor::new_without_display_handle(),
                )
                .await
            }
            "webgl2" => {
                let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
                desc.backends = wgpu::Backends::GL;
                wgpu::Instance::new(desc)
            }
            other => return Err(JsError::new(&format!("unknown backend {other:?}"))),
        };
        let (width, height) = (canvas.width().max(1), canvas.height().max(1));
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsError::new(&format!("could not create canvas surface: {e}")))?;
        let renderer = Renderer::with_surface(&instance, surface, width, height)
            .await
            .map_err(js_err)?;
        Ok(Engine {
            canvas,
            renderer,
            logical_size: (width, height),
            // Replaced by configure(); the Python Window always passes its required filter.
            filter: Filter::Nearest,
            timestep: FixedTimestep::new(1.0 / 60.0),
            textures: Vec::new(),
            sprites: Vec::new(),
            lists: Vec::new(),
        })
    }

    /// Graphics backend actually in use (for the spike report).
    #[wasm_bindgen(getter)]
    pub fn backend_name(&self) -> String {
        self.renderer.backend_name()
    }

    /// Size the canvas to `width`×`height` CSS pixels and set the window's filter and timestep.
    pub fn configure(
        &mut self,
        width: u32,
        height: u32,
        filter: &str,
        update_rate: f64,
    ) -> Result<(), JsError> {
        self.filter =
            parse_filter(filter)?.ok_or_else(|| JsError::new("the window filter is required"))?;
        self.timestep = FixedTimestep::new(update_rate);
        self.logical_size = (width.max(1), height.max(1));
        let (pw, ph) = self.physical_size();
        self.canvas.set_width(pw);
        self.canvas.set_height(ph);
        let style = self.canvas.style();
        let _ = style.set_property("width", &format!("{width}px"));
        let _ = style.set_property("height", &format!("{height}px"));
        self.renderer.resize(pw, ph);
        Ok(())
    }

    /// Add elapsed real time; returns how many `on_update` calls to run now.
    pub fn advance(&mut self, elapsed_seconds: f64) -> u32 {
        self.timestep.advance(elapsed_seconds)
    }

    pub fn begin_frame(&self) {
        frame::begin();
    }

    /// Queue a clear to an RGBA color with 0-255 components.
    pub fn clear(&self, r: i32, g: i32, b: i32, a: i32) -> Result<(), JsError> {
        let rgba = color::parse_rgba(&[r as i64, g as i64, b as i64, a as i64]).map_err(js_err)?;
        frame::push(FrameCommand::Clear(color::to_linear_f32(rgba)), "clear").map_err(js_err)
    }

    /// Render the commands queued since `begin_frame`.
    pub fn end_frame(&mut self) -> Result<(), JsError> {
        let commands = frame::end();
        self.renderer
            .render_frame(
                &commands,
                &mut store::global(),
                self.logical_size,
                self.filter,
            )
            .map_err(js_err)
    }

    /// Decode an image (bytes read by Python from Pyodide's filesystem), cached by path.
    pub fn load_texture(&mut self, path: &str, bytes: &[u8]) -> Result<u32, JsError> {
        let data = texture::load_from_bytes(Path::new(path), bytes).map_err(js_err)?;
        self.textures.push(TextureRef { data, filter: None });
        Ok((self.textures.len() - 1) as u32)
    }

    /// `[width, height]` of a texture in pixels.
    pub fn texture_size(&self, texture: u32) -> Result<Vec<u32>, JsError> {
        let t = self.texture(texture)?;
        Ok(vec![t.data.width, t.data.height])
    }

    pub fn create_sprite(
        &mut self,
        texture: u32,
        filter: &str,
        center_x: f64,
        center_y: f64,
        scale: f64,
    ) -> Result<u32, JsError> {
        let texture = TextureRef {
            data: self.texture(texture)?.data.clone(),
            filter: parse_filter(filter)?,
        };
        let id = store::global()
            .create_sprite(texture, center_x, center_y, scale)
            .map_err(js_err)?;
        self.sprites.push(Some(id));
        Ok((self.sprites.len() - 1) as u32)
    }

    /// Create a solid rectangle or circle (`kind`); `width`/`height` are the size (a circle
    /// uses `width` as its diameter). `filter` overrides the window filter for circles.
    #[allow(clippy::too_many_arguments)]
    pub fn create_shape_sprite(
        &mut self,
        kind: &str,
        width: f64,
        height: f64,
        filter: &str,
        center_x: f64,
        center_y: f64,
        r: i32,
        g: i32,
        b: i32,
        a: i32,
    ) -> Result<u32, JsError> {
        let rgba = color::parse_rgba(&[r as i64, g as i64, b as i64, a as i64]).map_err(js_err)?;
        let mut store = store::global();
        let id = match kind {
            "rectangle" => store.create_rectangle(width, height, rgba, center_x, center_y),
            "circle" => {
                store.create_circle(width / 2.0, rgba, center_x, center_y, parse_filter(filter)?)
            }
            other => return Err(JsError::new(&format!("unknown shape {other:?}"))),
        }
        .map_err(js_err)?;
        drop(store);
        self.sprites.push(Some(id));
        Ok((self.sprites.len() - 1) as u32)
    }

    /// `[velocity_x, velocity_y, angular_velocity]` of a sprite.
    pub fn get_velocity(&self, sprite: u32) -> Result<Vec<f64>, JsError> {
        let id = self.sprite(sprite)?;
        let store = store::global();
        let s = store.sprite(id).map_err(js_err)?;
        Ok(vec![s.velocity_x, s.velocity_y, s.angular_velocity])
    }

    pub fn set_velocity(&self, sprite: u32, vx: f64, vy: f64) -> Result<(), JsError> {
        store::global()
            .set_velocity(self.sprite(sprite)?, vx, vy)
            .map_err(js_err)
    }

    pub fn set_angular_velocity(&self, sprite: u32, value: f64) -> Result<(), JsError> {
        store::global()
            .set_angular_velocity(self.sprite(sprite)?, value)
            .map_err(js_err)
    }

    /// Move every sprite in a list by its velocities over `dt` seconds.
    pub fn update_list(&self, list: u32, dt: f64) -> Result<(), JsError> {
        store::global()
            .update_list(self.list(list)?, dt)
            .map_err(js_err)
    }

    /// `[r, g, b, a]` tint of a sprite.
    pub fn get_color(&self, sprite: u32) -> Result<Vec<u8>, JsError> {
        let id = self.sprite(sprite)?;
        let store = store::global();
        Ok(store.sprite(id).map_err(js_err)?.color.to_vec())
    }

    pub fn set_color(&self, sprite: u32, r: i32, g: i32, b: i32, a: i32) -> Result<(), JsError> {
        let rgba = color::parse_rgba(&[r as i64, g as i64, b as i64, a as i64]).map_err(js_err)?;
        store::global()
            .set_color(self.sprite(sprite)?, rgba)
            .map_err(js_err)
    }

    /// Draw calls in the most recent frame (for the browser checks).
    pub fn draw_calls(&self) -> u32 {
        self.renderer.last_frame_stats().draw_calls
    }

    pub fn destroy_sprite(&mut self, sprite: u32) {
        if let Some(slot) = self.sprites.get_mut(sprite as usize)
            && let Some(id) = slot.take()
        {
            store::global().destroy_sprite(id);
        }
    }

    /// `[center_x, center_y, scale, angle]`.
    pub fn get_sprite(&self, sprite: u32) -> Result<Vec<f64>, JsError> {
        let id = self.sprite(sprite)?;
        let store = store::global();
        let s = store.sprite(id).map_err(js_err)?;
        Ok(vec![s.center_x, s.center_y, s.scale, s.angle])
    }

    pub fn set_center_x(&self, sprite: u32, value: f64) -> Result<(), JsError> {
        store::global()
            .set_center_x(self.sprite(sprite)?, value)
            .map_err(js_err)
    }

    pub fn set_center_y(&self, sprite: u32, value: f64) -> Result<(), JsError> {
        store::global()
            .set_center_y(self.sprite(sprite)?, value)
            .map_err(js_err)
    }

    pub fn set_position(&self, sprite: u32, x: f64, y: f64) -> Result<(), JsError> {
        store::global()
            .set_position(self.sprite(sprite)?, x, y)
            .map_err(js_err)
    }

    pub fn set_scale(&self, sprite: u32, value: f64) -> Result<(), JsError> {
        store::global()
            .set_scale(self.sprite(sprite)?, value)
            .map_err(js_err)
    }

    pub fn set_angle(&self, sprite: u32, value: f64) -> Result<(), JsError> {
        store::global()
            .set_angle(self.sprite(sprite)?, value)
            .map_err(js_err)
    }

    pub fn create_list(&mut self) -> u32 {
        self.lists.push(Some(store::global().create_list()));
        (self.lists.len() - 1) as u32
    }

    pub fn destroy_list(&mut self, list: u32) {
        if let Some(slot) = self.lists.get_mut(list as usize)
            && let Some(id) = slot.take()
        {
            store::global().destroy_list(id);
        }
    }

    pub fn append(&self, list: u32, sprite: u32) -> Result<(), JsError> {
        store::global()
            .append(self.list(list)?, self.sprite(sprite)?)
            .map_err(js_err)
    }

    pub fn remove(&self, list: u32, sprite: u32) -> Result<(), JsError> {
        store::global()
            .remove(self.list(list)?, self.sprite(sprite)?)
            .map_err(js_err)
    }

    /// Queue a list for drawing. Only valid between `begin_frame` and `end_frame`.
    pub fn draw_list(&self, list: u32) -> Result<(), JsError> {
        frame::push(FrameCommand::DrawList(self.list(list)?), "draw").map_err(js_err)
    }
}
