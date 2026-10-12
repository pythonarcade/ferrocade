//! GPU instance buffer for one sprite list, plus its draw batches.

use std::ops::Range;
use std::sync::Arc;

use crate::instance::Instance;
use crate::store::ListState;
use crate::texture::{Filter, TextureData};

const INSTANCE_SIZE: u64 = std::mem::size_of::<Instance>() as u64;

/// Consecutive sprites drawn with one call: same texture, same effective filter.
pub struct Batch {
    pub texture: Arc<TextureData>,
    pub filter: Filter,
    pub instances: Range<u32>,
}

pub struct GpuList {
    buffer: Option<wgpu::Buffer>,
    /// Capacity of `buffer`, in instances.
    capacity: usize,
    batches: Vec<Batch>,
    /// (structure_version, window filter) the batches were built for.
    batches_key: Option<(u64, Filter)>,
}

impl GpuList {
    pub fn new() -> Self {
        Self {
            buffer: None,
            capacity: 0,
            batches: Vec::new(),
            batches_key: None,
        }
    }

    /// Bring the GPU copy up to date, uploading only what changed.
    pub fn sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        list: &mut ListState,
        window_filter: Filter,
    ) {
        let len = list.instances().len();
        let dirty = list.take_dirty();
        if len > self.capacity || self.buffer.is_none() {
            // Grow by doubling; a new buffer starts empty, so upload everything.
            self.capacity = len.max(self.capacity * 2).max(16);
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ferrocade sprite instances"),
                size: self.capacity as u64 * INSTANCE_SIZE,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.write(queue, list.instances(), 0..len);
        } else if let Some(range) = dirty {
            let range = range.start.min(len)..range.end.min(len);
            self.write(queue, list.instances(), range);
        }

        let key = (list.structure_version(), window_filter);
        if self.batches_key != Some(key) {
            self.batches = build_batches(list, window_filter);
            self.batches_key = Some(key);
        }
    }

    fn write(&self, queue: &wgpu::Queue, instances: &[Instance], range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        if let Some(buffer) = &self.buffer {
            queue.write_buffer(
                buffer,
                range.start as u64 * INSTANCE_SIZE,
                bytemuck::cast_slice(&instances[range]),
            );
        }
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn batches(&self) -> &[Batch] {
        &self.batches
    }
}

fn build_batches(list: &ListState, window_filter: Filter) -> Vec<Batch> {
    let mut batches: Vec<Batch> = Vec::new();
    for (i, texture) in list.slot_textures().iter().enumerate() {
        let filter = texture.effective_filter(window_filter);
        let i = i as u32;
        match batches.last_mut() {
            Some(b) if b.texture.id == texture.data.id && b.filter == filter => {
                b.instances.end = i + 1;
            }
            _ => batches.push(Batch {
                texture: texture.data.clone(),
                filter,
                instances: i..i + 1,
            }),
        }
    }
    batches
}
