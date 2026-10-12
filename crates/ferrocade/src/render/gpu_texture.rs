//! GPU copies of textures, uploaded on first use, with one bind group per (texture, filter).

use std::collections::HashMap;

use crate::texture::{Filter, TextureData, TextureId};

pub struct GpuTextures {
    layout: wgpu::BindGroupLayout,
    nearest: wgpu::Sampler,
    linear: wgpu::Sampler,
    views: HashMap<TextureId, wgpu::TextureView>,
    bind_groups: HashMap<(TextureId, Filter), wgpu::BindGroup>,
}

impl GpuTextures {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ferrocade texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = |filter: wgpu::FilterMode, label| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        Self {
            nearest: sampler(wgpu::FilterMode::Nearest, "ferrocade nearest sampler"),
            linear: sampler(wgpu::FilterMode::Linear, "ferrocade linear sampler"),
            layout,
            views: HashMap::new(),
            bind_groups: HashMap::new(),
        }
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    /// The bind group for an already-uploaded texture.
    pub fn get(&self, id: TextureId, filter: Filter) -> Option<&wgpu::BindGroup> {
        self.bind_groups.get(&(id, filter))
    }

    /// Upload the texture if needed and return its bind group for `filter`.
    pub fn bind_group(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &TextureData,
        filter: Filter,
    ) -> &wgpu::BindGroup {
        let view = self
            .views
            .entry(texture.id)
            .or_insert_with(|| upload(device, queue, texture));
        self.bind_groups
            .entry((texture.id, filter))
            .or_insert_with(|| {
                let sampler = match filter {
                    Filter::Nearest => &self.nearest,
                    Filter::Linear => &self.linear,
                };
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("ferrocade texture bind group"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                })
            })
    }
}

fn upload(device: &wgpu::Device, queue: &wgpu::Queue, texture: &TextureData) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: texture.width,
        height: texture.height,
        depth_or_array_layers: 1,
    };
    let gpu = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ferrocade texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        gpu.as_image_copy(),
        &texture.pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * texture.width),
            rows_per_image: Some(texture.height),
        },
        size,
    );
    gpu.create_view(&wgpu::TextureViewDescriptor::default())
}
