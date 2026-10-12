//! Central sprite store: all sprite and sprite-list data lives here, in Rust.
//!
//! Python `Sprite` and `SpriteList` objects are thin handles holding an ID. Each list keeps its
//! sprites' GPU instance data ready to upload; changing a sprite rewrites its instance in every
//! list that contains it and marks just that slot dirty, so drawing an unchanged list does no
//! per-sprite work at all.

use std::ops::Range;
use std::sync::{LazyLock, Mutex, MutexGuard};

use slotmap::SlotMap;
use smallvec::SmallVec;

use crate::instance::Instance;
use crate::texture::{self, Filter, TextureRef};
use crate::{Error, Result};

/// Default tint: leaves textures unchanged.
pub const WHITE: [u8; 4] = [255, 255, 255, 255];

slotmap::new_key_type! {
    pub struct SpriteId;
    pub struct ListId;
}

#[derive(Debug)]
pub struct SpriteState {
    pub center_x: f64,
    pub center_y: f64,
    pub scale: f64,
    pub angle: f64,
    pub texture: TextureRef,
    /// Tint, RGBA 0-255 (sRGB). White leaves the texture unchanged.
    pub color: [u8; 4],
    /// Pixels per second, applied by `update_list`.
    pub velocity_x: f64,
    pub velocity_y: f64,
    /// Degrees per second (positive is counter-clockwise), applied by `update_list`.
    pub angular_velocity: f64,
    /// Own size for shape sprites; `None` means "use the texture's size".
    pub base_size: Option<(f64, f64)>,
    /// Texture region to draw (u0, v0, u1, v1).
    pub uv_rect: [f32; 4],
    /// Every list containing this sprite, with the sprite's slot in that list.
    memberships: SmallVec<[(ListId, u32); 2]>,
}

impl SpriteState {
    /// Drawn size in pixels: base size (or texture size) × |scale|.
    pub fn drawn_size(&self) -> (f64, f64) {
        let (w, h) = self.base_size.unwrap_or((
            self.texture.data.width as f64,
            self.texture.data.height as f64,
        ));
        let scale = self.scale.abs();
        (w * scale, h * scale)
    }

    pub fn is_shape(&self) -> bool {
        self.base_size.is_some()
    }
}

#[derive(Debug, Default)]
pub struct ListState {
    sprites: Vec<SpriteId>,
    /// `instances[i]` mirrors `sprites[i]`, in GPU layout.
    instances: Vec<Instance>,
    slot_textures: Vec<TextureRef>,
    /// Slots changed since the renderer last uploaded this list.
    dirty: Option<Range<usize>>,
    /// Bumped whenever the order, membership, or textures change (batches must be rebuilt).
    structure_version: u64,
}

impl ListState {
    pub fn sprites(&self) -> &[SpriteId] {
        &self.sprites
    }

    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    pub fn slot_textures(&self) -> &[TextureRef] {
        &self.slot_textures
    }

    pub fn dirty(&self) -> Option<Range<usize>> {
        self.dirty.clone()
    }

    pub fn structure_version(&self) -> u64 {
        self.structure_version
    }

    /// Return and clear the dirty range (called by the renderer when it uploads).
    pub fn take_dirty(&mut self) -> Option<Range<usize>> {
        self.dirty.take()
    }

    fn mark_dirty(&mut self, range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        self.dirty = Some(match self.dirty.take() {
            Some(d) => d.start.min(range.start)..d.end.max(range.end),
            None => range,
        });
    }
}

#[derive(Default)]
pub struct SpriteStore {
    sprites: SlotMap<SpriteId, SpriteState>,
    lists: SlotMap<ListId, ListState>,
}

static GLOBAL: LazyLock<Mutex<SpriteStore>> = LazyLock::new(|| Mutex::new(SpriteStore::new()));

/// The process-wide store used by the Python bindings and the game loop.
pub fn global() -> MutexGuard<'static, SpriteStore> {
    GLOBAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn check_finite(name: &str, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::InvalidArgument(format!(
            "{name} must be a finite number, got {value}"
        )))
    }
}

impl SpriteStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_sprite(
        &mut self,
        texture: TextureRef,
        center_x: f64,
        center_y: f64,
        scale: f64,
    ) -> Result<SpriteId> {
        Ok(self.sprites.insert(SpriteState {
            center_x: check_finite("center_x", center_x)?,
            center_y: check_finite("center_y", center_y)?,
            scale: check_finite("scale", scale)?,
            angle: 0.0,
            texture,
            color: WHITE,
            velocity_x: 0.0,
            velocity_y: 0.0,
            angular_velocity: 0.0,
            base_size: None,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            memberships: SmallVec::new(),
        }))
    }

    /// Create a sprite with its own size and texture region (shape sprites).
    pub fn create_shape(
        &mut self,
        texture: TextureRef,
        uv_rect: [f32; 4],
        base_size: (f64, f64),
        color: [u8; 4],
        center_x: f64,
        center_y: f64,
    ) -> Result<SpriteId> {
        for (name, v) in [("width", base_size.0), ("height", base_size.1)] {
            if !(v.is_finite() && v > 0.0) {
                return Err(Error::InvalidArgument(format!(
                    "shape {name} must be a finite number > 0, got {v}"
                )));
            }
        }
        let id = self.create_sprite(texture, center_x, center_y, 1.0)?;
        let sprite = &mut self.sprites[id];
        sprite.uv_rect = uv_rect;
        sprite.base_size = Some(base_size);
        sprite.color = color;
        Ok(id)
    }

    /// A solid rectangle using the built-in shape texture.
    pub fn create_rectangle(
        &mut self,
        width: f64,
        height: f64,
        color: [u8; 4],
        center_x: f64,
        center_y: f64,
    ) -> Result<SpriteId> {
        let texture = TextureRef {
            data: texture::builtin_shapes(),
            filter: None,
        };
        self.create_shape(
            texture,
            texture::RECT_UV,
            (width, height),
            color,
            center_x,
            center_y,
        )
    }

    /// A solid circle using the built-in shape texture; `filter` overrides the window filter.
    pub fn create_circle(
        &mut self,
        radius: f64,
        color: [u8; 4],
        center_x: f64,
        center_y: f64,
        filter: Option<Filter>,
    ) -> Result<SpriteId> {
        if !(radius.is_finite() && radius > 0.0) {
            return Err(Error::InvalidArgument(format!(
                "circle radius must be a finite number > 0, got {radius}"
            )));
        }
        let texture = TextureRef {
            data: texture::builtin_shapes(),
            filter,
        };
        let d = radius * 2.0;
        self.create_shape(
            texture,
            texture::CIRCLE_UV,
            (d, d),
            color,
            center_x,
            center_y,
        )
    }

    /// Delete a sprite, removing it from every list that contains it.
    pub fn destroy_sprite(&mut self, id: SpriteId) {
        let lists: SmallVec<[ListId; 2]> = match self.sprites.get(id) {
            Some(sprite) => sprite.memberships.iter().map(|&(list, _)| list).collect(),
            None => return,
        };
        for list in lists {
            let _ = self.remove(list, id);
        }
        self.sprites.remove(id);
    }

    pub fn sprite(&self, id: SpriteId) -> Result<&SpriteState> {
        self.sprites
            .get(id)
            .ok_or_else(|| Error::InvalidArgument("sprite no longer exists".into()))
    }

    pub fn create_list(&mut self) -> ListId {
        self.lists.insert(ListState::default())
    }

    /// Delete a list, dropping its memberships from all of its sprites.
    pub fn destroy_list(&mut self, id: ListId) {
        if let Some(list) = self.lists.remove(id) {
            for sprite in list.sprites {
                if let Some(s) = self.sprites.get_mut(sprite) {
                    s.memberships.retain(|(l, _)| *l != id);
                }
            }
        }
    }

    pub fn list(&self, id: ListId) -> Result<&ListState> {
        self.lists
            .get(id)
            .ok_or_else(|| Error::InvalidArgument("sprite list no longer exists".into()))
    }

    pub fn list_mut(&mut self, id: ListId) -> Result<&mut ListState> {
        self.lists
            .get_mut(id)
            .ok_or_else(|| Error::InvalidArgument("sprite list no longer exists".into()))
    }

    pub fn contains_list(&self, id: ListId) -> bool {
        self.lists.contains_key(id)
    }

    /// Add a sprite to the end of a list (drawn on top of earlier sprites).
    pub fn append(&mut self, list_id: ListId, sprite_id: SpriteId) -> Result<()> {
        let sprite = self
            .sprites
            .get_mut(sprite_id)
            .ok_or_else(|| Error::InvalidArgument("sprite no longer exists".into()))?;
        if sprite.memberships.iter().any(|(l, _)| *l == list_id) {
            return Err(Error::InvalidArgument(
                "sprite is already in this sprite list".into(),
            ));
        }
        let list = self
            .lists
            .get_mut(list_id)
            .ok_or_else(|| Error::InvalidArgument("sprite list no longer exists".into()))?;
        let slot = list.sprites.len();
        sprite.memberships.push((list_id, slot as u32));
        list.sprites.push(sprite_id);
        list.instances.push(Instance::from_sprite(sprite));
        list.slot_textures.push(sprite.texture.clone());
        list.mark_dirty(slot..slot + 1);
        list.structure_version += 1;
        Ok(())
    }

    /// Remove a sprite from a list, keeping the order of the remaining sprites.
    pub fn remove(&mut self, list_id: ListId, sprite_id: SpriteId) -> Result<()> {
        let not_member = || Error::InvalidArgument("sprite is not in this sprite list".into());
        let sprite = self.sprites.get_mut(sprite_id).ok_or_else(not_member)?;
        let pos = sprite
            .memberships
            .iter()
            .position(|(l, _)| *l == list_id)
            .ok_or_else(not_member)?;
        let (_, slot) = sprite.memberships.remove(pos);
        let slot = slot as usize;

        let list = self.lists.get_mut(list_id).ok_or_else(not_member)?;
        list.sprites.remove(slot);
        list.instances.remove(slot);
        list.slot_textures.remove(slot);
        // Every later sprite moved down one slot.
        for (new_slot, &moved) in list.sprites.iter().enumerate().skip(slot) {
            if let Some(m) = self.sprites[moved]
                .memberships
                .iter_mut()
                .find(|(l, _)| *l == list_id)
            {
                m.1 = new_slot as u32;
            }
        }
        let len = list.sprites.len();
        list.mark_dirty(slot..len);
        list.structure_version += 1;
        Ok(())
    }

    pub fn contains(&self, list_id: ListId, sprite_id: SpriteId) -> bool {
        self.sprites
            .get(sprite_id)
            .is_some_and(|s| s.memberships.iter().any(|(l, _)| *l == list_id))
    }

    pub fn set_center_x(&mut self, id: SpriteId, value: f64) -> Result<()> {
        let value = check_finite("center_x", value)?;
        self.update(id, |s| s.center_x = value)
    }

    pub fn set_center_y(&mut self, id: SpriteId, value: f64) -> Result<()> {
        let value = check_finite("center_y", value)?;
        self.update(id, |s| s.center_y = value)
    }

    pub fn set_position(&mut self, id: SpriteId, x: f64, y: f64) -> Result<()> {
        let x = check_finite("center_x", x)?;
        let y = check_finite("center_y", y)?;
        self.update(id, |s| {
            s.center_x = x;
            s.center_y = y;
        })
    }

    pub fn set_scale(&mut self, id: SpriteId, value: f64) -> Result<()> {
        let value = check_finite("scale", value)?;
        self.update(id, |s| s.scale = value)
    }

    pub fn set_angle(&mut self, id: SpriteId, value: f64) -> Result<()> {
        let value = check_finite("angle", value)?;
        self.update(id, |s| s.angle = value)
    }

    // Velocities aren't drawn, so setting them doesn't touch instances or dirty ranges.

    pub fn set_velocity_x(&mut self, id: SpriteId, value: f64) -> Result<()> {
        let value = check_finite("velocity_x", value)?;
        self.sprite_mut(id)?.velocity_x = value;
        Ok(())
    }

    pub fn set_velocity_y(&mut self, id: SpriteId, value: f64) -> Result<()> {
        let value = check_finite("velocity_y", value)?;
        self.sprite_mut(id)?.velocity_y = value;
        Ok(())
    }

    pub fn set_velocity(&mut self, id: SpriteId, vx: f64, vy: f64) -> Result<()> {
        let vx = check_finite("velocity_x", vx)?;
        let vy = check_finite("velocity_y", vy)?;
        let sprite = self.sprite_mut(id)?;
        sprite.velocity_x = vx;
        sprite.velocity_y = vy;
        Ok(())
    }

    pub fn set_angular_velocity(&mut self, id: SpriteId, value: f64) -> Result<()> {
        let value = check_finite("angular_velocity", value)?;
        self.sprite_mut(id)?.angular_velocity = value;
        Ok(())
    }

    /// Advance every moving sprite in `list` by its velocities over `dt` seconds.
    ///
    /// Sprites with zero velocity are skipped, so they cost nothing to redraw. A moving
    /// sprite's instance is rewritten in every list that contains it.
    pub fn update_list(&mut self, list: ListId, dt: f64) -> Result<()> {
        if !(dt.is_finite() && dt >= 0.0) {
            return Err(Error::InvalidArgument(format!(
                "delta_time must be a finite number >= 0, got {dt}"
            )));
        }
        let members = self.list(list)?.sprites.clone();
        for id in members {
            let s = &self.sprites[id];
            if s.velocity_x == 0.0 && s.velocity_y == 0.0 && s.angular_velocity == 0.0 {
                continue;
            }
            let (vx, vy, va) = (s.velocity_x, s.velocity_y, s.angular_velocity);
            self.update(id, |s| {
                s.center_x += vx * dt;
                s.center_y += vy * dt;
                s.angle += va * dt;
            })?;
        }
        Ok(())
    }

    fn sprite_mut(&mut self, id: SpriteId) -> Result<&mut SpriteState> {
        self.sprites
            .get_mut(id)
            .ok_or_else(|| Error::InvalidArgument("sprite no longer exists".into()))
    }

    pub fn set_color(&mut self, id: SpriteId, color: [u8; 4]) -> Result<()> {
        self.update(id, |s| s.color = color)
    }

    pub fn set_texture(&mut self, id: SpriteId, texture: TextureRef) -> Result<()> {
        self.update(id, |s| s.texture = texture)?;
        let sprite = &self.sprites[id];
        for &(list_id, slot) in &sprite.memberships {
            let list = &mut self.lists[list_id];
            list.slot_textures[slot as usize] = sprite.texture.clone();
            list.structure_version += 1;
        }
        Ok(())
    }

    /// Apply a change to a sprite, then rewrite its instance in every list containing it.
    fn update(&mut self, id: SpriteId, change: impl FnOnce(&mut SpriteState)) -> Result<()> {
        let sprite = self
            .sprites
            .get_mut(id)
            .ok_or_else(|| Error::InvalidArgument("sprite no longer exists".into()))?;
        change(sprite);
        let instance = Instance::from_sprite(sprite);
        for &(list_id, slot) in &sprite.memberships {
            let list = &mut self.lists[list_id];
            let slot = slot as usize;
            list.instances[slot] = instance;
            list.mark_dirty(slot..slot + 1);
        }
        Ok(())
    }
}
