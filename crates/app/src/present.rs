//! Shows GM's back buffer in a window, scaled by the largest whole number that fits, centred on black.
//! Whole-number scaling keeps every GM pixel the same size; anything else would make some rows and
//! columns thicker than others. A window smaller than the picture (a short browser window) shrinks it to
//! fit instead, with nearest sampling, rather than cutting it off.

use crate::render::Renderer;

/// The scale at which `content` shows in `window`: the largest whole number that fits, or below 1 the
/// largest that fits at all; and the top-left offset that centres it.
pub fn fit(content: (u32, u32), window: (u32, u32)) -> (f64, (i32, i32)) {
    let (cw, ch) = (content.0.max(1), content.1.max(1));
    let whole = (window.0 / cw).min(window.1 / ch);
    let scale = if whole >= 1 {
        f64::from(whole)
    } else {
        (f64::from(window.0) / f64::from(cw)).min(f64::from(window.1) / f64::from(ch))
    };
    let offset = |win: u32, c: u32| ((f64::from(win) - f64::from(c) * scale) / 2.0).floor() as i32;
    (scale, (offset(window.0, cw), offset(window.1, ch)))
}

/// A window position (physical pixels) as a pixel of `content` shown there by [`fit`]: GM's window
/// pixels, which `mouse_x` maps through the views, as GM8 maps a scaled window's cursor back into its
/// region. Outside the picture it keeps going (negative, or past the edge), as GM's mouse does outside
/// the region.
pub fn window_to_content(pos: (f64, f64), content: (u32, u32), window: (u32, u32)) -> (i32, i32) {
    let (scale, (x, y)) = fit(content, window);
    let map = |p: f64, offset: i32| ((p - f64::from(offset)) / scale).floor() as i32;
    (map(pos.0, x), map(pos.1, y))
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    offset: [f32; 2],
    scale: f32,
    pad: f32,
}

pub struct Presenter {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    /// Rebuilt when the renderer replaces its back buffer (tracked by serial).
    bind_group: Option<(u64, wgpu::BindGroup)>,
}

impl Presenter {
    pub fn new(
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        size: (u32, u32),
    ) -> Result<Presenter, String> {
        let caps = surface.get_capabilities(adapter);
        // GM never converts colour spaces, so write values to the window as they are.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or("the window surface supports no formats")?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: Default::default(),
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(device, &config);

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("present"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("present"),
            source: wgpu::ShaderSource::Wgsl(include_str!("present.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("present"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("present"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_present"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_present"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("present"),
            size: size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Presenter {
            surface,
            config,
            pipeline,
            layout,
            uniform,
            bind_group: None,
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        self.config.width = size.0;
        self.config.height = size.1;
        self.surface.configure(device, &self.config);
    }

    /// The next window texture. An outdated or lost swapchain is reconfigured and asked once more, so a
    /// resize doesn't cost a frame.
    fn acquire(&mut self, device: &wgpu::Device) -> Option<wgpu::SurfaceTexture> {
        for _ in 0..2 {
            match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t)
                | wgpu::CurrentSurfaceTexture::Suboptimal(t) => return Some(t),
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                    self.surface.configure(device, &self.config);
                }
                // Minimised, occluded or timed out: skip this frame.
                _ => return None,
            }
        }
        None
    }

    /// Shows the renderer's back buffer. Returns false when nothing could be shown (minimised, occluded,
    /// or nothing rendered yet).
    pub fn present(&mut self, renderer: &Renderer) -> bool {
        let Some((screen, serial)) = renderer.screen() else {
            return false;
        };
        let device = renderer.device();
        let queue = renderer.queue();
        let Some(frame) = self.acquire(device) else {
            return false;
        };

        if self.bind_group.as_ref().is_none_or(|(s, _)| *s != serial) {
            let view = screen.create_view(&Default::default());
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("present"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.uniform.as_entire_binding(),
                    },
                ],
            });
            self.bind_group = Some((serial, group));
        }
        let (scale, (x, y)) = fit(
            (screen.width(), screen.height()),
            (self.config.width, self.config.height),
        );
        let uniform = Uniform {
            offset: [x as f32, y as f32],
            scale: scale as f32,
            pad: 0.0,
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniform));

        let view = frame.texture.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("present"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group.as_ref().expect("just made").1, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
        queue.present(frame);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_whole_multiples_and_centres() {
        assert_eq!(fit((320, 240), (640, 480)), (2.0, (0, 0)));
        assert_eq!(fit((320, 240), (1000, 800)), (3.0, (20, 40)));
        assert_eq!(fit((960, 720), (1000, 800)), (1.0, (20, 40)));
    }

    #[test]
    fn the_cursor_maps_back_into_the_pictures_pixels() {
        // 960x720 (screenScale 3) in a 2000x1600 window: scale 2, offset (40, 80).
        let at = |x, y| window_to_content((x, y), (960, 720), (2000, 1600));
        assert_eq!(at(40.0, 80.0), (0, 0));
        assert_eq!(at(41.9, 81.9), (0, 0));
        assert_eq!(at(42.0, 82.0), (1, 1));
        assert_eq!(at(1959.9, 1519.9), (959, 719));
        // In the black border: outside the picture, as GM reports a cursor outside its region.
        assert_eq!(at(0.0, 0.0), (-20, -40));
        // Shrunk to fit (scale 0.5): a window pixel covers two of the picture's.
        assert_eq!(
            window_to_content((101.0, 0.0), (320, 240), (160, 240)),
            (202, -120)
        );
    }

    #[test]
    fn shrinks_to_fit_a_smaller_window() {
        // Height-bound: 600 / 720.
        let (scale, (x, y)) = fit((960, 720), (1000, 600));
        assert_eq!(scale, 600.0 / 720.0);
        assert_eq!((x, y), (100, 0));
        assert_eq!(fit((320, 240), (160, 240)), (0.5, (0, 60)));
    }
}
