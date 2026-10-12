//! SPIKE: browser bindings for Ferrocade (Phase 1b, approach B).
//!
//! Exposes the engine core to JavaScript via wasm-bindgen; a pure-Python shim running in
//! Pyodide calls this API. Only compiled for `wasm32-unknown-unknown`, so desktop workspace
//! builds are unaffected.
//!
//! IDs cross the boundary as `u32` handles into tables here (slotmap keys are 64-bit, and JS
//! numbers are only exact to 2^53).

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use ferrocade::collision::{ColliderId, Shape};
use ferrocade::color;
use ferrocade::frame::{self, FrameCommand};
use ferrocade::input::{Key, Keyboard, RawKeyEvent};
use ferrocade::render::Renderer;
use ferrocade::store::{self, ListId, SpriteId};
use ferrocade::texture::{self, Filter, TextureRef};
use ferrocade::timestep::FixedTimestep;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent};

fn js_err(e: ferrocade::Error) -> JsError {
    JsError::new(&e.to_string())
}

fn parse_shape(kind: &str, a: f64, b: f64) -> Result<Shape, JsError> {
    match kind {
        "circle" => Ok(Shape::Circle { radius: a }),
        "box" => Ok(Shape::Box {
            width: a,
            height: b,
        }),
        "aabb" => Ok(Shape::Aabb {
            width: a,
            height: b,
        }),
        other => Err(JsError::new(&format!("unknown collider kind {other:?}"))),
    }
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
    stats: ferrocade::window::WindowState,
    last_frame: Option<web_time::Instant>,
    textures: Vec<TextureRef>,
    sprites: Vec<Option<SpriteId>>,
    lists: Vec<Option<ListId>>,
    colliders: Vec<Option<ColliderId>>,
    /// Shared with the canvas key listeners.
    keyboard: Rc<RefCell<Keyboard>>,
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

    fn collider(&self, handle: u32) -> Result<ColliderId, JsError> {
        self.colliders
            .get(handle as usize)
            .copied()
            .flatten()
            .ok_or_else(|| JsError::new("collider no longer exists"))
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
        let keyboard = Rc::new(RefCell::new(Keyboard::default()));
        attach_keyboard(&canvas, &keyboard)?;
        Ok(Engine {
            canvas,
            renderer,
            logical_size: (width, height),
            // Replaced by configure(); the Python Window always passes its required filter.
            filter: Filter::Nearest,
            timestep: FixedTimestep::new(1.0 / 60.0),
            stats: ferrocade::window::WindowState::new(width, height),
            last_frame: None,
            textures: Vec::new(),
            sprites: Vec::new(),
            lists: Vec::new(),
            colliders: Vec::new(),
            keyboard,
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
            .map_err(js_err)?;
        let now = web_time::Instant::now();
        let interval = self.last_frame.map_or(0.0, |t| (now - t).as_secs_f64());
        self.last_frame = Some(now);
        let s = self.renderer.last_frame_stats();
        self.stats
            .record_frame(interval, s.render_cpu_ms, s.draw_calls, s.instances);
        Ok(())
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

    /// `[fps, frame_ms, render_cpu_ms, draw_calls, instances]` averaged over ~1 s.
    pub fn frame_stats(&self) -> Vec<f64> {
        let s = self.stats.frame_stats();
        vec![
            s.fps,
            s.frame_ms,
            s.render_cpu_ms,
            s.draw_calls as f64,
            s.instances as f64,
        ]
    }

    /// Key events since the last call, as flat `[code, pressed (0/1), modifiers, ...]` triples.
    pub fn take_key_events(&self) -> Vec<i32> {
        self.keyboard
            .borrow_mut()
            .drain()
            .into_iter()
            .flat_map(|e| [e.key.code() as i32, e.pressed as i32, e.modifiers as i32])
            .collect()
    }

    /// Give the canvas keyboard focus.
    pub fn focus(&self) {
        let _ = self.canvas.focus();
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

    /// Create a collider: `kind` is "circle" (`a` = radius), "box" or "aabb" (`a` × `b`).
    pub fn create_collider(
        &mut self,
        kind: &str,
        a: f64,
        b: f64,
        offset_x: f64,
        offset_y: f64,
    ) -> Result<u32, JsError> {
        let shape = parse_shape(kind, a, b)?;
        let id = store::global()
            .create_collider(shape, (offset_x, offset_y))
            .map_err(js_err)?;
        self.colliders.push(Some(id));
        Ok((self.colliders.len() - 1) as u32)
    }

    pub fn set_collider_shape(
        &self,
        collider: u32,
        kind: &str,
        a: f64,
        b: f64,
    ) -> Result<(), JsError> {
        store::global()
            .set_collider_shape(self.collider(collider)?, parse_shape(kind, a, b)?)
            .map_err(js_err)
    }

    pub fn set_collider_offset(&self, collider: u32, x: f64, y: f64) -> Result<(), JsError> {
        store::global()
            .set_collider_offset(self.collider(collider)?, (x, y))
            .map_err(js_err)
    }

    pub fn destroy_collider(&mut self, collider: u32) {
        if let Some(slot) = self.colliders.get_mut(collider as usize)
            && let Some(id) = slot.take()
        {
            store::global().destroy_collider(id);
        }
    }

    /// Attach a collider to a sprite (`collider` = -1 detaches).
    pub fn set_sprite_collider(&self, sprite: u32, collider: i32) -> Result<(), JsError> {
        let collider = if collider < 0 {
            None
        } else {
            Some(self.collider(collider as u32)?)
        };
        store::global()
            .attach_collider(self.sprite(sprite)?, collider)
            .map_err(js_err)
    }

    pub fn collides(&self, a: u32, b: u32) -> Result<bool, JsError> {
        store::global()
            .collides(self.sprite(a)?, self.sprite(b)?)
            .map_err(js_err)
    }

    /// Slots in `list` touching `sprite`, ascending.
    pub fn collides_with_list(&self, sprite: u32, list: u32) -> Result<Vec<u32>, JsError> {
        store::global()
            .sprite_vs_list(self.sprite(sprite)?, self.list(list)?)
            .map_err(js_err)
    }

    /// Touching pairs as flat `[slot_a, slot_b, ...]`.
    pub fn collisions_with(&self, list_a: u32, list_b: u32) -> Result<Vec<u32>, JsError> {
        let pairs = store::global()
            .list_vs_list(self.list(list_a)?, self.list(list_b)?)
            .map_err(js_err)?;
        Ok(pairs.into_iter().flat_map(|(i, j)| [i, j]).collect())
    }

    /// Touching pairs within `list` as flat `[earlier, later, ...]`.
    pub fn collisions(&self, list: u32) -> Result<Vec<u32>, JsError> {
        let pairs = store::global()
            .within_list(self.list(list)?)
            .map_err(js_err)?;
        Ok(pairs.into_iter().flat_map(|(i, j)| [i, j]).collect())
    }

    /// Queue a list for drawing. Only valid between `begin_frame` and `end_frame`.
    pub fn draw_list(&self, list: u32) -> Result<(), JsError> {
        frame::push(FrameCommand::DrawList(self.list(list)?), "draw").map_err(js_err)
    }
}

/// Feed the canvas's key events into `keyboard`, and release held keys when focus is lost.
/// The listeners live as long as the page (the engine is never dropped).
fn attach_keyboard(
    canvas: &HtmlCanvasElement,
    keyboard: &Rc<RefCell<Keyboard>>,
) -> Result<(), JsError> {
    let js = |e: JsValue| JsError::new(&format!("could not attach key listeners: {e:?}"));
    // Focusable, so it receives key events (the loader focuses it on start and on click).
    canvas.set_tab_index(0);

    for (event_name, pressed) in [("keydown", true), ("keyup", false)] {
        let keyboard = keyboard.clone();
        let listener = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            let Some(key) = Key::from_w3c(&event.code()) else {
                return;
            };
            keyboard.borrow_mut().push(RawKeyEvent {
                key,
                pressed,
                repeat: event.repeat(),
            });
            // Stop the page from acting on game keys (scrolling, quick-find), but keep browser
            // shortcuts (Ctrl/Alt/Meta combinations, F1-F12) and Tab focus navigation.
            let browser_key = event.ctrl_key()
                || event.alt_key()
                || event.meta_key()
                || key == Key::Tab
                || (Key::F1.code()..=Key::F12.code()).contains(&key.code());
            if !browser_key {
                event.prevent_default();
            }
        });
        canvas
            .add_event_listener_with_callback(event_name, listener.as_ref().unchecked_ref())
            .map_err(js)?;
        listener.forget();
    }

    let release = {
        let keyboard = keyboard.clone();
        Closure::<dyn FnMut()>::new(move || keyboard.borrow_mut().release_all())
    };
    canvas
        .add_event_listener_with_callback("blur", release.as_ref().unchecked_ref())
        .map_err(js)?;
    if let Some(window) = web_sys::window() {
        window
            .add_event_listener_with_callback("blur", release.as_ref().unchecked_ref())
            .map_err(js)?;
        if let Some(document) = window.document() {
            let hidden = {
                let keyboard = keyboard.clone();
                let document = document.clone();
                Closure::<dyn FnMut()>::new(move || {
                    if document.visibility_state() == web_sys::VisibilityState::Hidden {
                        keyboard.borrow_mut().release_all();
                    }
                })
            };
            document
                .add_event_listener_with_callback(
                    "visibilitychange",
                    hidden.as_ref().unchecked_ref(),
                )
                .map_err(js)?;
            hidden.forget();
        }
    }
    release.forget();
    Ok(())
}
