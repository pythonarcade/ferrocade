//! The sprite pipeline: draws sprite lists as instanced, textured quads.

use std::collections::HashMap;

use super::gpu_list::GpuList;
use super::gpu_texture::GpuTextures;
use crate::instance::Instance;
use crate::store::{ListId, SpriteStore};
use crate::texture::Filter;

/// Work done by the most recent frame (for tests and performance displays).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    pub draw_calls: u32,
    pub instances: u32,
}

pub struct SpritePipeline {
    pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    textures: GpuTextures,
    lists: HashMap<ListId, GpuList>,
    stats: FrameStats,
}

impl SpritePipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("sprite.wgsl"));
        let textures = GpuTextures::new(device);

        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ferrocade globals"),
            size: std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ferrocade globals layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ferrocade globals bind group"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ferrocade sprite pipeline layout"),
            bind_group_layouts: &[Some(&globals_layout), Some(textures.layout())],
            ..Default::default()
        });
        let attribute = |location, format, offset: usize| wgpu::VertexAttribute {
            format,
            offset: offset as u64,
            shader_location: location,
        };
        let attributes = [
            attribute(
                0,
                wgpu::VertexFormat::Float32x2,
                std::mem::offset_of!(Instance, center),
            ),
            attribute(
                1,
                wgpu::VertexFormat::Float32x2,
                std::mem::offset_of!(Instance, size),
            ),
            attribute(
                2,
                wgpu::VertexFormat::Float32,
                std::mem::offset_of!(Instance, angle),
            ),
            attribute(
                3,
                wgpu::VertexFormat::Float32x4,
                std::mem::offset_of!(Instance, uv_rect),
            ),
            attribute(
                4,
                wgpu::VertexFormat::Unorm8x4,
                std::mem::offset_of!(Instance, color),
            ),
        ];
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &attributes,
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ferrocade sprite pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(instance_layout)],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            globals,
            globals_bind_group,
            textures,
            lists: HashMap::new(),
            stats: FrameStats::default(),
        }
    }

    /// Per-frame setup: projection for the current window size; free GPU data of deleted lists.
    pub fn begin_frame(
        &mut self,
        queue: &wgpu::Queue,
        logical_size: (u32, u32),
        store: &SpriteStore,
    ) {
        let (w, h) = (logical_size.0.max(1) as f32, logical_size.1.max(1) as f32);
        // Orthographic: x 0..w and y 0..h (origin bottom-left, y up) to clip space -1..1.
        let projection: [[f32; 4]; 4] = [
            [2.0 / w, 0.0, 0.0, 0.0],
            [0.0, 2.0 / h, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, -1.0, 0.0, 1.0],
        ];
        queue.write_buffer(&self.globals, 0, bytemuck::cast_slice(&projection));
        self.stats = FrameStats::default();
        self.lists.retain(|id, _| store.contains_list(*id));
    }

    /// Upload a list's changes and make sure its textures are on the GPU.
    pub fn prepare_list(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        store: &mut SpriteStore,
        id: ListId,
        window_filter: Filter,
    ) {
        let Ok(list) = store.list_mut(id) else {
            return;
        };
        let gpu = self.lists.entry(id).or_insert_with(GpuList::new);
        gpu.sync(device, queue, list, window_filter);
        for batch in gpu.batches() {
            self.textures
                .bind_group(device, queue, &batch.texture, batch.filter);
        }
    }

    pub fn stats(&self) -> FrameStats {
        self.stats
    }

    pub fn draw_list(&mut self, pass: &mut wgpu::RenderPass<'_>, id: ListId) {
        let Some(gpu) = self.lists.get(&id) else {
            return;
        };
        let Some(buffer) = gpu.buffer() else {
            return;
        };
        if gpu.batches().is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        for batch in gpu.batches() {
            if let Some(bind_group) = self.textures.get(batch.texture.id, batch.filter) {
                pass.set_bind_group(1, bind_group, &[]);
                pass.draw(0..6, batch.instances.clone());
                self.stats.draw_calls += 1;
                self.stats.instances += batch.instances.len() as u32;
            }
        }
    }
}
