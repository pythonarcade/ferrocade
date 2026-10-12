//! wgpu renderer: owns the GPU device and executes each frame's draw commands.
//!
//! It never touches windowing directly: the platform layer creates the surface and passes it
//! in, or the renderer draws to an offscreen texture (used by tests).

use crate::color;
use crate::frame::FrameCommand;
use crate::texture::Filter;
use crate::{Error, Result};

const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

enum Target {
    Surface {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
        /// False while the window has zero size (minimized).
        configured: bool,
    },
    Offscreen {
        texture: wgpu::Texture,
        width: u32,
        height: u32,
    },
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: Target,
}

impl Renderer {
    /// Create a renderer drawing to a window surface of the given physical size.
    pub async fn with_surface(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let adapter = request_adapter(instance, Some(&surface)).await?;
        let (device, queue) = request_device(&adapter).await?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| Error::Gpu("surface has no supported formats".into()))?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            // Fifo (vsync) is the one present mode every backend supports.
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        let mut renderer = Self {
            device,
            queue,
            target: Target::Surface {
                surface,
                config,
                configured: false,
            },
        };
        renderer.resize(width, height);
        Ok(renderer)
    }

    /// Create a renderer drawing to an offscreen texture (for tests and headless rendering).
    pub async fn offscreen(width: u32, height: u32) -> Result<Self> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = request_adapter(&instance, None).await?;
        let (device, queue) = request_device(&adapter).await?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ferrocade offscreen target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Ok(Self {
            device,
            queue,
            target: Target::Offscreen {
                texture,
                width,
                height,
            },
        })
    }

    /// Reconfigure the surface for a new physical size. A zero size (minimized) pauses drawing.
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Target::Surface {
            surface,
            config,
            configured,
        } = &mut self.target
        {
            if width == 0 || height == 0 {
                *configured = false;
                return;
            }
            config.width = width;
            config.height = height;
            surface.configure(&self.device, config);
            *configured = true;
        }
    }

    /// Draw one frame from the commands recorded during `on_draw`.
    ///
    /// `logical_size` is the window size in logical pixels (the coordinate space sprites use);
    /// `window_filter` applies to textures without their own filter override.
    pub fn render_frame(
        &mut self,
        commands: &[FrameCommand],
        logical_size: (u32, u32),
        window_filter: Filter,
    ) -> Result<()> {
        let _ = (logical_size, window_filter);
        let (view, surface_texture) = match self.acquire()? {
            Some(acquired) => acquired,
            None => return Ok(()), // Nothing to draw into this frame (minimized or surface lost).
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ferrocade frame"),
            });

        // A clear only takes effect when the next pass starts, so consecutive clears collapse
        // into the last one. Frames without a clear start from the default background.
        let mut pending_clear = Some(color::to_linear_f32(color::DARK_GRAY));
        for command in commands {
            match *command {
                FrameCommand::Clear(rgba) => pending_clear = Some(rgba),
                FrameCommand::DrawList(_) => {
                    let pass = begin_pass(&mut encoder, &view, pending_clear.take());
                    drop(pass);
                }
            }
        }
        if let Some(rgba) = pending_clear {
            drop(begin_pass(&mut encoder, &view, Some(rgba)));
        }

        self.queue.submit([encoder.finish()]);
        if let Some(surface_texture) = surface_texture {
            self.queue.present(surface_texture);
        }
        Ok(())
    }

    /// Read back the offscreen target as tightly packed RGBA8 rows, top row first.
    pub fn read_pixels(&self) -> Result<Vec<u8>> {
        let Target::Offscreen {
            texture,
            width,
            height,
        } = &self.target
        else {
            return Err(Error::Gpu("read_pixels() requires an offscreen renderer".into()));
        };
        let (width, height) = (*width, *height);
        let unpadded = width * 4;
        let padded = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ferrocade readback"),
            size: (padded * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            texture.size(),
        );
        self.queue.submit([encoder.finish()]);

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| Error::Gpu(e.to_string()))?;
        let data = slice
            .get_mapped_range()
            .map_err(|e| Error::Gpu(format!("could not map readback buffer: {e:?}")))?;
        let mut pixels = Vec::with_capacity((unpadded * height) as usize);
        for row in data.chunks(padded as usize) {
            pixels.extend_from_slice(&row[..unpadded as usize]);
        }
        Ok(pixels)
    }

    /// Get a view to draw into this frame, or `None` if the frame should be skipped.
    fn acquire(&mut self) -> Result<Option<(wgpu::TextureView, Option<wgpu::SurfaceTexture>)>> {
        match &mut self.target {
            Target::Offscreen { texture, .. } => Ok(Some((
                texture.create_view(&wgpu::TextureViewDescriptor::default()),
                None,
            ))),
            Target::Surface {
                surface,
                config,
                configured,
            } => {
                if !*configured {
                    return Ok(None);
                }
                use wgpu::CurrentSurfaceTexture as Current;
                match surface.get_current_texture() {
                    Current::Success(frame) => {
                        let view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        Ok(Some((view, Some(frame))))
                    }
                    Current::Suboptimal(frame) => {
                        // Draw this frame anyway; reconfigure for the next one.
                        let view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());
                        surface.configure(&self.device, config);
                        Ok(Some((view, Some(frame))))
                    }
                    Current::Outdated => {
                        surface.configure(&self.device, config);
                        Ok(None)
                    }
                    Current::Timeout | Current::Occluded | Current::Validation => Ok(None),
                    Current::Lost => Err(Error::Gpu("the window surface was lost".into())),
                }
            }
        }
    }
}

fn begin_pass<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    clear: Option<[f64; 4]>,
) -> wgpu::RenderPass<'e> {
    let load = match clear {
        Some([r, g, b, a]) => wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
        None => wgpu::LoadOp::Load,
    };
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("ferrocade pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        // 2D only: no depth buffer; draw order is the layering.
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

async fn request_adapter(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'static>>,
) -> Result<wgpu::Adapter> {
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: surface,
            ..Default::default()
        })
        .await
        .map_err(|e| Error::Gpu(format!("no compatible graphics adapter: {e}")))
}

async fn request_device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue)> {
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("ferrocade device"),
            // Stay within WebGL2 limits so the same renderer works on the web.
            required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                .using_resolution(adapter.limits()),
            ..Default::default()
        })
        .await
        .map_err(|e| Error::Gpu(format!("could not create graphics device: {e}")))
}
