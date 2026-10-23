//! Turns [`draw::Frame`]s into GPU work.
//!
//! The draw list is replayed on the CPU into one vertex buffer of clip-space triangles and lines, split into
//! as few draw calls as possible: a new one starts only when the target, blend mode, primitive type,
//! scissor or sampled texture changes. Sprites, tiles, backgrounds and text all sample the same bind
//! group (the sprite and background array textures) and untextured shapes can join any batch, so a room
//! of thousands of instances is a handful of draw calls.
//!
//! The renderer owns GM's back buffer (the "screen") and every surface. Each frame is rendered exactly
//! once; presenting it again at display rate is [`crate::present`]'s job.

use crate::textures::{Bank, Layers, SpriteTex, TexRect, Textures};
use draw::{BlendFactor, BlendMode, Color, Command, DrawState, Frame, Rect, SurfaceId, Target};
use std::collections::HashMap;
use std::ops::Range;

/// The format of GM's back buffer and surfaces. Not sRGB: GM does no colour-space conversion.
pub const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// `Vertex::layer` for plain colour.
const UNTEXTURED: u32 = u32::MAX;
/// Set in `Vertex::layer` for the background array (binding 1) instead of the sprite one (binding 0).
const BACKGROUND_BANK: u32 = 1 << 31;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    /// Clip space.
    pos: [f32; 2],
    /// Texels.
    uv: [f32; 2],
    color: [u8; 4],
    /// The sampled image's texels: x0, y0, x1, y1 (exclusive).
    rect: [u16; 4],
    layer: u32,
}

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4, 3 => Uint16x4, 4 => Uint32
];

/// A render target: the screen or a surface.
struct RenderTarget {
    texture: wgpu::Texture,
    /// For rendering into.
    view: wgpu::TextureView,
    /// For sampling from (`draw_surface`): the surface as a one-layer array in the sprite slot, so it fits
    /// the sprite pipeline.
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
    /// Unique per target ever created, so batches can tell sources apart.
    serial: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PipelineKey {
    /// `None` replaces instead of blending (`draw_clear`).
    blend: Option<BlendMode>,
    lines: bool,
}

/// What a batch samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    /// Nothing; can join a batch with any source.
    None,
    Images,
    /// A surface, by serial.
    Surface(u64),
}

struct Pass {
    view: wgpu::TextureView,
    /// The target's serial: a new pass starts whenever the target changes.
    serial: u64,
    batches: Vec<Batch>,
}

struct Batch {
    key: PipelineKey,
    source: Source,
    bind_group: Option<wgpu::BindGroup>,
    scissor: [u32; 4],
    vertices: Range<u32>,
}

impl Batch {
    /// Whether submitting issues a draw call for this batch: it has vertices and a visible scissor.
    fn is_drawn(&self) -> bool {
        let [_, _, w, h] = self.scissor;
        !self.vertices.is_empty() && w > 0 && h > 0
    }
}

/// GM's drawing state resolved for one draw: the target's size and the port that actually applies.
struct State {
    target: Target,
    target_size: (u32, u32),
    view: Rect,
    port: Rect,
    blend: BlendMode,
}

impl State {
    fn resolve(gm: &DrawState, width: u32, height: u32) -> State {
        let port = match gm.target {
            Target::Screen => gm.port,
            // Direct3D 8 ignores the viewport while a surface is the target: the view fills the surface.
            Target::Surface(_) => Rect::new(0, 0, width as i32, height as i32),
        };
        State {
            target: gm.target,
            target_size: (width, height),
            view: gm.view,
            port,
            blend: gm.blend,
        }
    }
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    sprites: Vec<Option<SpriteTex>>,
    backgrounds: Vec<Option<TexRect>>,
    images: wgpu::BindGroup,
    bind_layout: wgpu::BindGroupLayout,
    /// The background array, also bound beside surfaces.
    background_view: wgpu::TextureView,
    shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    screen: Option<RenderTarget>,
    /// The picture GM last showed: a copy of the back buffer at the last [`Command::Present`].
    shown: Option<RenderTarget>,
    surfaces: HashMap<SurfaceId, RenderTarget>,
    next_serial: u64,
    vertex_buffer: wgpu::Buffer,
    // Per-frame scratch, kept to reuse the allocations.
    vertices: Vec<Vertex>,
    passes: Vec<Pass>,
    presents: Vec<Present>,
    warned_self_draw: bool,
}

/// A [`Command::Present`]: copy the back buffer into the shown picture once `after` passes have run.
struct Present {
    after: usize,
    from: wgpu::Texture,
    to: wgpu::Texture,
    size: wgpu::Extent3d,
}

impl Renderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, textures: Textures) -> Renderer {
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        };
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("images"),
            entries: &[texture_entry(0), texture_entry(1)],
        });
        let sprite_view = upload_layers(&device, &queue, &textures.sprite_pages, "sprites");
        let background_view =
            upload_layers(&device, &queue, &textures.background_pages, "backgrounds");
        let images = images_bind_group(&device, &bind_layout, &sprite_view, &background_view);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("draw"),
            source: wgpu::ShaderSource::Wgsl(include_str!("draw.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("draw"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let vertex_buffer = vertex_buffer(&device, 1 << 16);

        Renderer {
            device,
            queue,
            sprites: textures.sprites,
            backgrounds: textures.backgrounds,
            images,
            bind_layout,
            background_view,
            shader,
            pipeline_layout,
            pipelines: HashMap::new(),
            screen: None,
            shown: None,
            surfaces: HashMap::new(),
            next_serial: 0,
            vertex_buffer,
            vertices: Vec::new(),
            passes: Vec::new(),
            presents: Vec::new(),
            warned_self_draw: false,
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// What the window shows: GM's back buffer as it was at the last [`Command::Present`], with a serial
    /// that changes whenever the texture is replaced. `None` before the first present.
    pub fn screen(&self) -> Option<(&wgpu::Texture, u64)> {
        self.shown.as_ref().map(|s| (&s.texture, s.serial))
    }

    /// Renders one frame into the back buffer and surfaces.
    pub fn render(&mut self, frame: &Frame) {
        let (mut width, mut height) = (frame.width.max(1), frame.height.max(1));
        self.resize_screen(width, height);
        // Surfaces this renderer missed the creation of (a loaded savestate) start blank; ones the game has
        // since freed go.
        self.surfaces
            .retain(|id, _| frame.surfaces.iter().any(|s| s.id == *id));
        for s in &frame.surfaces {
            let have = self.surfaces.get(&s.id).map(|t| (t.width, t.height));
            if have != Some((s.width.max(1), s.height.max(1))) {
                let surface = self.create_surface(s.width, s.height);
                self.surfaces.insert(s.id, surface);
            }
        }

        self.vertices.clear();
        self.passes.clear();
        self.presents.clear();
        // GM's state carries over from the previous step; the frame says what it was.
        let mut gm = frame.start;
        for command in &frame.commands {
            match command {
                // A replaced or freed surface's texture lives on while recorded passes use it (wgpu
                // reference-counts it).
                Command::CreateSurface { id, width, height } => {
                    let surface = self.create_surface(*width, *height);
                    self.surfaces.insert(*id, surface);
                }
                Command::FreeSurface(id) => {
                    self.surfaces.remove(id);
                }
                // Passes already recorded keep drawing into the old texture (wgpu reference-counts it).
                Command::ResizeScreen {
                    width: w,
                    height: h,
                } => {
                    (width, height) = ((*w).max(1), (*h).max(1));
                    self.resize_screen(width, height);
                }
                Command::Present => {
                    let Some(screen) = &self.screen else {
                        continue;
                    };
                    let (from, w, h) = (screen.texture.clone(), screen.width, screen.height);
                    if self
                        .shown
                        .as_ref()
                        .is_none_or(|s| (s.width, s.height) != (w, h))
                    {
                        // One layer, like the screen: the presenter samples it as a plain 2D texture.
                        self.shown = Some(self.create_target(w, h, 1, "shown"));
                    }
                    let to = self.shown.as_ref().expect("made above").texture.clone();
                    let size = wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    };
                    let after = self.passes.len();
                    self.presents.push(Present {
                        after,
                        from,
                        to,
                        size,
                    });
                }
                Command::SetTarget(_) | Command::SetView { .. } | Command::SetBlend(_) => {
                    let surfaces = &self.surfaces;
                    gm.apply(command, (width, height), |id| {
                        surfaces.get(&id).map(|s| (s.width, s.height))
                    });
                }
                command => {
                    // The target is looked up by id on every draw: a surface freed while it's the
                    // target swallows draws until the next SetTarget, and one re-created under the same
                    // id receives them.
                    let target = match gm.target {
                        Target::Screen => self.screen.as_ref(),
                        Target::Surface(id) => self.surfaces.get(&id),
                    };
                    let Some(target) = target else {
                        continue;
                    };
                    let state = State::resolve(&gm, target.width, target.height);
                    // A present splits the passes, so it copies what was drawn before it.
                    let presented = self
                        .presents
                        .last()
                        .is_some_and(|p| p.after == self.passes.len());
                    if presented || self.passes.last().map(|p| p.serial) != Some(target.serial) {
                        self.passes.push(Pass {
                            view: target.view.clone(),
                            serial: target.serial,
                            batches: Vec::new(),
                        });
                    }
                    self.draw(&state, command);
                    let end = self.vertices.len() as u32;
                    if let Some(batch) = self.passes.last_mut().and_then(|p| p.batches.last_mut()) {
                        batch.vertices.end = end;
                    }
                }
            }
        }

        self.submit();
    }

    /// Replaces the back buffer if its size differs.
    fn resize_screen(&mut self, width: u32, height: u32) {
        if self
            .screen
            .as_ref()
            .is_none_or(|s| (s.width, s.height) != (width, height))
        {
            // One layer: the presenter samples it as a plain 2D texture.
            let screen = self.create_target(width, height, 1, "screen");
            self.screen = Some(screen);
        }
    }

    /// A surface: drawn through the array binding, unlike the screen and the shown frame.
    fn create_surface(&mut self, width: u32, height: u32) -> RenderTarget {
        let layers = array_layers(&self.device, 1);
        self.create_target(width, height, layers, "surface")
    }

    /// A render target drawn to through its layer 0. `layers` is 1, or 2 where [`array_layers`] says so.
    fn create_target(&mut self, width: u32, height: u32, layers: u32, label: &str) -> RenderTarget {
        // Emitters keep surfaces within draw::MAX_SURFACE_SIZE; clamp anyway rather than fail the device.
        let max = self.device.limits().max_texture_dimension_2d;
        let (width, height) = (width.clamp(1, max), height.clamp(1, max));
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            array_layer_count: Some(1),
            ..Default::default()
        });
        let sampled = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let bind_group = images_bind_group(
            &self.device,
            &self.bind_layout,
            &sampled,
            &self.background_view,
        );
        self.next_serial += 1;
        RenderTarget {
            texture,
            view,
            bind_group,
            width,
            height,
            serial: self.next_serial,
        }
    }

    fn draw(&mut self, state: &State, command: &Command) {
        let tf = Transform::new(state);
        let color = |c: Color, alpha: f64| {
            let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
            [c.r(), c.g(), c.b(), a]
        };
        let tris = PipelineKey {
            blend: Some(state.blend),
            lines: false,
        };
        let lines = PipelineKey {
            blend: Some(state.blend),
            lines: true,
        };
        match command {
            Command::Clear { color: c, alpha } => {
                // Fill the whole port without blending; the scissor keeps it inside.
                let key = PipelineKey {
                    blend: None,
                    lines: false,
                };
                self.begin_batch(state, key, Source::None, None);
                let v = |x: f32, y: f32| untextured([x, y], color(*c, *alpha));
                let quad = [v(-1.0, 1.0), v(1.0, 1.0), v(1.0, -1.0), v(-1.0, -1.0)];
                push_quad(&mut self.vertices, quad);
            }
            Command::Sprite(s) => {
                let Some(sprite) = self.sprite(s.sprite) else {
                    return;
                };
                let Some(&frame) = sprite
                    .frames
                    .get(s.frame.rem_euclid(sprite.frames.len().max(1) as i32) as usize)
                else {
                    return;
                };
                let origin = (f64::from(sprite.origin_x), f64::from(sprite.origin_y));
                let part = (0.0, 0.0, f64::from(frame.w), f64::from(frame.h));
                let place = Placement {
                    x: s.x,
                    y: s.y,
                    xscale: s.xscale,
                    yscale: s.yscale,
                    angle: s.angle,
                    origin,
                };
                self.begin_batch(state, tris, Source::Images, None);
                push_image(
                    &mut self.vertices,
                    &tf,
                    frame,
                    part,
                    place,
                    color(s.blend, s.alpha),
                );
            }
            Command::Background(b) => {
                let Some(rect) = self.background(b.background) else {
                    return;
                };
                let (w, h) = (f64::from(rect.w) * b.xscale, f64::from(rect.h) * b.yscale);
                let part = (0.0, 0.0, f64::from(rect.w), f64::from(rect.h));
                let view = state.view;
                // GM tiles from the room's left edge to past the view, with half the view's other
                // dimension to spare for rotated views.
                let end_x = f64::from(view.x + view.w + view.h / 2);
                let end_y = f64::from(view.y + view.h + view.w / 2);
                let xs = tile_positions(b.x, w, b.tile_h, f64::from(view.x), end_x);
                let ys = tile_positions(b.y, h, b.tile_v, f64::from(view.y), end_y);
                self.begin_batch(state, tris, Source::Images, None);
                for y in ys {
                    for x in xs.clone() {
                        let place = Placement {
                            x,
                            y,
                            xscale: b.xscale,
                            yscale: b.yscale,
                            angle: 0.0,
                            origin: (0.0, 0.0),
                        };
                        push_image(
                            &mut self.vertices,
                            &tf,
                            rect,
                            part,
                            place,
                            color(b.blend, b.alpha),
                        );
                    }
                }
            }
            Command::Tile(t) => {
                let Some(rect) = self.background(t.background) else {
                    return;
                };
                let part = (
                    f64::from(t.left),
                    f64::from(t.top),
                    f64::from(t.width),
                    f64::from(t.height),
                );
                let place = Placement {
                    x: t.x,
                    y: t.y,
                    xscale: t.xscale,
                    yscale: t.yscale,
                    angle: 0.0,
                    origin: (0.0, 0.0),
                };
                self.begin_batch(state, tris, Source::Images, None);
                push_image(
                    &mut self.vertices,
                    &tf,
                    rect,
                    part,
                    place,
                    color(t.blend, t.alpha),
                );
            }
            Command::Rectangle {
                x1,
                y1,
                x2,
                y2,
                color: c,
                alpha,
                outline,
            } => {
                let (x1, x2) = (x1.min(*x2), x1.max(*x2));
                let (y1, y2) = (y1.min(*y2), y1.max(*y2));
                // GM nudges whole-number far edges so they're filled too: both corners are inclusive.
                let nudge = |v: f64| if v == v.floor() { v + 0.01 } else { v };
                let (x2, y2) = (nudge(x2), nudge(y2));
                let rgba = color(*c, *alpha);
                let corners = [(x1, y1), (x2, y1), (x2, y2), (x1, y2)]
                    .map(|(x, y)| untextured(tf.apply(x, y), rgba));
                if *outline {
                    self.begin_batch(state, lines, Source::None, None);
                    for i in 0..4 {
                        self.vertices.extend([corners[i], corners[(i + 1) % 4]]);
                    }
                } else {
                    self.begin_batch(state, tris, Source::None, None);
                    push_quad(&mut self.vertices, corners);
                }
            }
            Command::Line {
                x1,
                y1,
                x2,
                y2,
                color: c,
                alpha,
            } => {
                let rgba = color(*c, *alpha);
                self.begin_batch(state, lines, Source::None, None);
                self.vertices.extend([
                    untextured(tf.apply(*x1, *y1), rgba),
                    untextured(tf.apply(*x2, *y2), rgba),
                ]);
            }
            Command::Circle {
                x,
                y,
                radius,
                color: c,
                alpha,
                outline,
            } => {
                let rgba = color(*c, *alpha);
                let n = draw::CIRCLE_PRECISION;
                // GM's fan: n + 1 rim points, the last one back at the start.
                #[allow(clippy::disallowed_methods, reason = "drawing only, never game state")]
                let rim = |i: u32| {
                    let a = f64::from(i) * std::f64::consts::TAU / f64::from(n);
                    untextured(tf.apply(x + radius * a.cos(), y + radius * a.sin()), rgba)
                };
                if *outline {
                    self.begin_batch(state, lines, Source::None, None);
                    for i in 0..n {
                        self.vertices.extend([rim(i), rim(i + 1)]);
                    }
                } else {
                    let centre = untextured(tf.apply(*x, *y), rgba);
                    self.begin_batch(state, tris, Source::None, None);
                    for i in 0..n {
                        self.vertices.extend([centre, rim(i), rim(i + 1)]);
                    }
                }
            }
            Command::Text(text) => {
                if self.sprite(text.font.sprite).is_none() {
                    return;
                }
                self.begin_batch(state, tris, Source::Images, None);
                let sprite = self.sprites[text.font.sprite as usize]
                    .as_ref()
                    .expect("checked above");
                let glyphs = text.font.layout(
                    &text.text,
                    sprite.frames.len() as u32,
                    sprite.width,
                    sprite.height,
                );
                // OpenGMK draws glyphs through the sprite's origin; the game's fonts have theirs at 0, 0.
                let origin = (f64::from(sprite.origin_x), f64::from(sprite.origin_y));
                let rgba = color(text.color, text.alpha);
                for g in glyphs {
                    let frame = sprite.frames[g.frame as usize];
                    let place = Placement {
                        x: text.x + f64::from(g.dx),
                        y: text.y + f64::from(g.dy),
                        xscale: 1.0,
                        yscale: 1.0,
                        angle: 0.0,
                        origin,
                    };
                    let part = (0.0, 0.0, f64::from(frame.w), f64::from(frame.h));
                    push_image(&mut self.vertices, &tf, frame, part, place, rgba);
                }
            }
            Command::Surface(s) => {
                let Some(surface) = self.surfaces.get(&s.surface) else {
                    return;
                };
                if state.target == Target::Surface(s.surface) {
                    // GM leaves this undefined; wgpu forbids it. Draw nothing.
                    if !self.warned_self_draw {
                        log_warning("a surface was drawn onto itself; skipped");
                        self.warned_self_draw = true;
                    }
                    return;
                }
                // The surface sits in the sprite slot of its own bind group.
                let rect = TexRect {
                    bank: Bank::Sprites,
                    layer: 0,
                    x: 0,
                    y: 0,
                    w: surface.width,
                    h: surface.height,
                };
                let source = Source::Surface(surface.serial);
                let bind_group = surface.bind_group.clone();
                let part = (0.0, 0.0, f64::from(rect.w), f64::from(rect.h));
                let place = Placement {
                    x: s.x,
                    y: s.y,
                    xscale: s.xscale,
                    yscale: s.yscale,
                    angle: 0.0,
                    origin: (0.0, 0.0),
                };
                self.begin_batch(state, tris, source, Some(bind_group));
                push_image(
                    &mut self.vertices,
                    &tf,
                    rect,
                    part,
                    place,
                    color(s.blend, s.alpha),
                );
            }
            Command::CreateSurface { .. }
            | Command::FreeSurface(_)
            | Command::ResizeScreen { .. }
            | Command::SetTarget(_)
            | Command::SetView { .. }
            | Command::SetBlend(_)
            | Command::Present => unreachable!("state commands are handled by render"),
        }
    }

    fn sprite(&self, id: u32) -> Option<&SpriteTex> {
        self.sprites.get(id as usize)?.as_ref()
    }

    fn background(&self, id: u32) -> Option<TexRect> {
        *self.backgrounds.get(id as usize)?
    }

    /// Makes sure the current batch can take vertices with `key` and `source`, starting a new one if not.
    fn begin_batch(
        &mut self,
        state: &State,
        key: PipelineKey,
        source: Source,
        bind_group: Option<wgpu::BindGroup>,
    ) {
        let scissor = scissor(state);
        let at = self.vertices.len() as u32;
        let pass = self.passes.last_mut().expect("render opens a pass first");
        if let Some(last) = pass.batches.last_mut() {
            let compatible = last.key == key
                && last.scissor == scissor
                && (source == Source::None || last.source == Source::None || last.source == source);
            if compatible && last.vertices.end == at {
                if last.source == Source::None && source != Source::None {
                    last.source = source;
                    last.bind_group = bind_group;
                }
                return;
            }
        }
        pass.batches.push(Batch {
            key,
            source,
            bind_group,
            scissor,
            vertices: at..at,
        });
        self.ensure_pipeline(key);
    }

    fn ensure_pipeline(&mut self, key: PipelineKey) {
        let (device, shader, layout) = (&self.device, &self.shader, &self.pipeline_layout);
        self.pipelines.entry(key).or_insert_with(|| {
            let blend = key.blend.map(|b| {
                // Direct3D 8 blends alpha with the same factors as colour.
                let component = wgpu::BlendComponent {
                    src_factor: blend_factor(b.src),
                    dst_factor: blend_factor(b.dest),
                    operation: wgpu::BlendOperation::Add,
                };
                wgpu::BlendState {
                    color: component,
                    alpha: component,
                }
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("draw"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &VERTEX_ATTRIBUTES,
                    })],
                },
                primitive: wgpu::PrimitiveState {
                    topology: if key.lines {
                        wgpu::PrimitiveTopology::LineList
                    } else {
                        wgpu::PrimitiveTopology::TriangleList
                    },
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: TARGET_FORMAT,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        });
    }

    fn submit(&mut self) {
        let bytes: &[u8] = bytemuck::cast_slice(&self.vertices);
        if bytes.len() as u64 > self.vertex_buffer.size() {
            self.vertex_buffer =
                vertex_buffer(&self.device, (bytes.len() as u64).next_power_of_two());
        }
        if !bytes.is_empty() {
            self.queue.write_buffer(&self.vertex_buffer, 0, bytes);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        let copy = |encoder: &mut wgpu::CommandEncoder, p: &Present| {
            encoder.copy_texture_to_texture(p.from.as_image_copy(), p.to.as_image_copy(), p.size);
        };
        let mut presents = self.presents.iter().peekable();
        for (i, pass) in self.passes.iter().enumerate() {
            while let Some(p) = presents.next_if(|p| p.after == i) {
                copy(&mut encoder, p);
            }
            if !pass.batches.iter().any(Batch::is_drawn) {
                continue;
            }
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("draw"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &pass.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            for batch in pass.batches.iter().filter(|b| b.is_drawn()) {
                let [x, y, w, h] = batch.scissor;
                rpass.set_pipeline(&self.pipelines[&batch.key]);
                rpass.set_bind_group(0, batch.bind_group.as_ref().unwrap_or(&self.images), &[]);
                rpass.set_scissor_rect(x, y, w, h);
                rpass.draw(batch.vertices.clone(), 0..1);
            }
        }
        for p in presents {
            copy(&mut encoder, p);
        }
        self.queue.submit([encoder.finish()]);
    }

    /// Number of draw calls the last frame needed (for diagnostics).
    pub fn draw_calls(&self) -> usize {
        self.passes
            .iter()
            .map(|p| p.batches.iter().filter(|b| b.is_drawn()).count())
            .sum()
    }

    /// Reads what the window shows ([`screen`](Self::screen)) back as RGBA8 rows. Blocks on the GPU, so
    /// it's native-only (screenshots).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_screen(&self) -> Result<(u32, u32, Vec<u8>), String> {
        let screen = self.shown.as_ref().ok_or("nothing presented yet")?;
        let (w, h) = (screen.width, screen.height);
        let row = (w * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(row * h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback"),
            });
        encoder.copy_texture_to_buffer(
            screen.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        rx.recv()
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        let data = buffer.get_mapped_range(..).map_err(|e| e.to_string())?;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for r in data.chunks_exact(row as usize) {
            rgba.extend_from_slice(&r[..(w * 4) as usize]);
        }
        Ok((w, h, rgba))
    }
}

#[derive(Clone, Copy)]
struct Placement {
    x: f64,
    y: f64,
    xscale: f64,
    yscale: f64,
    angle: f64,
    origin: (f64, f64),
}

/// Maps room coordinates through the current view and port to clip space.
struct Transform {
    sx: f64,
    sy: f64,
    ox: f64,
    oy: f64,
    tw: f64,
    th: f64,
}

impl Transform {
    fn new(state: &State) -> Transform {
        let (view, port) = (state.view, state.port);
        let ratio = |p: i32, v: i32| {
            if v == 0 {
                0.0
            } else {
                f64::from(p) / f64::from(v)
            }
        };
        let (sx, sy) = (ratio(port.w, view.w), ratio(port.h, view.h));
        // Direct3D puts pixel centres on whole coordinates; wgpu puts them half a pixel further on. Shifting
        // everything by half a target pixel makes the two agree, so coverage matches GM exactly.
        Transform {
            sx,
            sy,
            ox: f64::from(port.x) - f64::from(view.x) * sx + 0.5,
            oy: f64::from(port.y) - f64::from(view.y) * sy + 0.5,
            tw: f64::from(state.target_size.0),
            th: f64::from(state.target_size.1),
        }
    }

    fn apply(&self, x: f64, y: f64) -> [f32; 2] {
        let px = x * self.sx + self.ox;
        let py = y * self.sy + self.oy;
        [
            (px / self.tw * 2.0 - 1.0) as f32,
            (1.0 - py / self.th * 2.0) as f32,
        ]
    }
}

/// Most copies of a tiled background drawn along one axis; only reachable with absurdly small scales.
const MAX_TILE_COPIES: u32 = 4096;

/// Where copies of a background layer go along one axis. GM tiles from the first copy at or before the
/// room's edge (`pos` wrapped into `(-size, 0]`) until a copy starts at or past `end`. Copies that end
/// before `view_start` would be clipped, so they're skipped.
fn tile_positions(
    pos: f64,
    size: f64,
    tile: bool,
    view_start: f64,
    end: f64,
) -> impl Iterator<Item = f64> + Clone {
    let (start, count) = if !tile || size <= 0.0 {
        (pos, 1)
    } else {
        let p = pos.rem_euclid(size);
        let first = if p > 0.0 { p - size } else { p };
        // Index of the first copy that reaches into the view.
        let skip = ((view_start - first) / size).floor().max(0.0);
        let start = first + skip * size;
        let count = ((end - start) / size)
            .ceil()
            .clamp(1.0, f64::from(MAX_TILE_COPIES));
        (start, count as u32)
    };
    (0..count).map(move |k| start + f64::from(k) * size)
}

/// The current port clipped to the target, as a scissor rectangle.
fn scissor(state: &State) -> [u32; 4] {
    let (tw, th) = (state.target_size.0 as i32, state.target_size.1 as i32);
    let p = state.port;
    let x0 = p.x.clamp(0, tw);
    let y0 = p.y.clamp(0, th);
    let x1 = (p.x.saturating_add(p.w)).clamp(x0, tw);
    let y1 = (p.y.saturating_add(p.h)).clamp(y0, th);
    [x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32]
}

fn push_quad(vertices: &mut Vec<Vertex>, [a, b, c, d]: [Vertex; 4]) {
    vertices.extend([a, b, c, a, c, d]);
}

/// GM's `draw_sprite_general`: a textured quad of `part` of `image`, placed around its origin.
fn push_image(
    vertices: &mut Vec<Vertex>,
    tf: &Transform,
    image: TexRect,
    part: (f64, f64, f64, f64),
    place: Placement,
    color: [u8; 4],
) {
    let (px, py, pw, ph) = part;
    let (w, h) = (place.xscale * pw, place.yscale * ph);
    // GM shifts every image up and left by half a room pixel to counter Direct3D's pixel centres. The shift
    // is added after the origin is scaled (it isn't scaled itself) and before rotation, so a mirrored
    // sprite covers the same pixels as an unmirrored one.
    let left = -place.xscale * place.origin.0 - 0.5;
    let top = -place.yscale * place.origin.1 - 0.5;
    let (right, bottom) = (left + w, top + h);
    #[allow(clippy::disallowed_methods, reason = "drawing only, never game state")]
    let (sin, cos) = (-place.angle.to_radians()).sin_cos();
    let corner =
        |dx: f64, dy: f64| tf.apply(place.x + dx * cos - dy * sin, place.y + dy * cos + dx * sin);
    let (u0, v0) = (
        (f64::from(image.x) + px) as f32,
        (f64::from(image.y) + py) as f32,
    );
    let (u1, v1) = (u0 + pw as f32, v0 + ph as f32);
    let rect = [
        image.x as u16,
        image.y as u16,
        (image.x + image.w) as u16,
        (image.y + image.h) as u16,
    ];
    let v = |pos, uv| Vertex {
        pos,
        uv,
        color,
        rect,
        layer: match image.bank {
            Bank::Sprites => image.layer,
            Bank::Backgrounds => image.layer | BACKGROUND_BANK,
        },
    };
    push_quad(
        vertices,
        [
            v(corner(left, top), [u0, v0]),
            v(corner(right, top), [u1, v0]),
            v(corner(right, bottom), [u1, v1]),
            v(corner(left, bottom), [u0, v1]),
        ],
    );
}

fn untextured(pos: [f32; 2], color: [u8; 4]) -> Vertex {
    Vertex {
        pos,
        uv: [0.0; 2],
        color,
        rect: [0; 4],
        layer: UNTEXTURED,
    }
}

fn blend_factor(f: BlendFactor) -> wgpu::BlendFactor {
    use wgpu::BlendFactor as W;
    match f {
        BlendFactor::Zero => W::Zero,
        BlendFactor::One => W::One,
        BlendFactor::SrcColor => W::Src,
        BlendFactor::InvSrcColor => W::OneMinusSrc,
        BlendFactor::SrcAlpha => W::SrcAlpha,
        BlendFactor::InvSrcAlpha => W::OneMinusSrcAlpha,
        BlendFactor::DestAlpha => W::DstAlpha,
        BlendFactor::InvDestAlpha => W::OneMinusDstAlpha,
        BlendFactor::DestColor => W::Dst,
        BlendFactor::InvDestColor => W::OneMinusDst,
        BlendFactor::SrcAlphaSaturate => W::SrcAlphaSaturated,
    }
}

fn vertex_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vertices"),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// How many layers to give a texture of `layers` that's sampled through a `D2Array` view. WebGL2 makes a
/// one-layer texture a plain 2D one, which an array view can't sample (it reads black), so there it gets a
/// second, unused layer.
fn array_layers(device: &wgpu::Device, layers: u32) -> u32 {
    if device.adapter_info().backend == wgpu::Backend::Gl {
        layers.max(2)
    } else {
        layers
    }
}

/// Uploads an image array and returns a view of it for sampling.
fn upload_layers(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layers: &Layers,
    label: &str,
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: layers.width,
        height: layers.height,
        depth_or_array_layers: array_layers(device, layers.layers.len() as u32),
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (i, layer) in layers.layers.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: i as u32,
                },
                aspect: wgpu::TextureAspect::All,
            },
            layer,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(layers.width * 4),
                rows_per_image: Some(layers.height),
            },
            wgpu::Extent3d {
                depth_or_array_layers: 1,
                ..size
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    })
}

fn images_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sprites: &wgpu::TextureView,
    backgrounds: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("images"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(sprites),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(backgrounds),
            },
        ],
    })
}

pub(crate) fn log_warning(message: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("warning: {message}");
    #[cfg(target_arch = "wasm32")]
    let _ = message;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A renderer on the machine's GPU with no images, or `None` (the test is skipped) without one.
    fn gpu_renderer() -> Option<Renderer> {
        let empty = || Layers {
            width: 1,
            height: 1,
            layers: vec![vec![0; 4]],
        };
        let textures = Textures {
            sprite_pages: empty(),
            background_pages: empty(),
            sprites: Vec::new(),
            backgrounds: Vec::new(),
        };
        match pollster::block_on(crate::gpu::request(crate::gpu::instance(), None)) {
            Ok(gpu) => Some(Renderer::new(gpu.device, gpu.queue, textures)),
            Err(e) => {
                eprintln!("skipped: {e}");
                None
            }
        }
    }

    /// `oScreen`'s dark-level composition (851-3160), as `gm` records it: the draw stage has drawn the room
    /// into `screen` (surface 1); the next begin step fills `darkSurf` (2) with black and draws the lights
    /// in the light colour, multiplies `screen` by it (`bm_dest_color, bm_zero`), repairs `screen`'s alpha
    /// (`bm_src_color, bm_one` with a black rectangle), and blits it 2x onto the back buffer with
    /// `bm_one, bm_zero`. The renderer joins at this frame (as after a savestate load), so it learns the
    /// surfaces from [`Frame::surfaces`]. Set `SPELUNKY_SHOT=out.png` to look at the result.
    #[test]
    fn dark_level_composition_lights_only_the_circles() {
        let Some(mut renderer) = gpu_renderer() else {
            return;
        };
        let (screen, dark) = (SurfaceId(1), SurfaceId(2));
        let blend = |src, dest| Command::SetBlend(BlendMode::new(src, dest));
        let rect = |x1, y1, x2, y2, color, alpha| Command::Rectangle {
            x1,
            y1,
            x2,
            y2,
            color: Color(color),
            alpha,
            outline: false,
        };
        let circle = |x, y, radius, color| Command::Circle {
            x,
            y,
            radius,
            color: Color(color),
            alpha: 1.0,
            outline: false,
        };
        let blit = |surface, scale| {
            Command::Surface(draw::SurfaceDraw {
                surface,
                x: 0.0,
                y: 0.0,
                xscale: scale,
                yscale: scale,
                blend: Color::WHITE,
                alpha: 1.0,
            })
        };
        // A darkness of 0.5 tints the player's light: make_color_rgb(127, 127, 255).
        let light = 0xFF_7F_7F;
        let (level, wall) = (0x40_80_C0, 0xFF_FF_FF);
        let mut frame = Frame::new(640, 480);
        frame.start.target = Target::Surface(screen);
        frame.surfaces = (0..3)
            .map(|id| draw::SurfaceSize {
                id: SurfaceId(id),
                width: 320,
                height: 240,
            })
            .collect();
        frame.commands = vec![
            // The draw stage's room, and a translucent instance, which leaves screen's alpha under 1.
            Command::SetTarget(Target::Surface(screen)),
            Command::Clear {
                color: Color(level),
                alpha: 1.0,
            },
            rect(150.0, 110.0, 169.0, 129.0, wall, 0.5),
            // Begin step.
            Command::SetTarget(Target::Screen),
            Command::Clear {
                color: Color::BLACK,
                alpha: 1.0,
            },
            Command::SetTarget(Target::Surface(dark)),
            rect(0.0, 0.0, 320.0, 240.0, 0, 1.0),
            circle(160.0, 120.0, 64.0, light),
            circle(40.0, 40.0, 16.0, light),
            Command::SetTarget(Target::Surface(screen)),
            blend(BlendFactor::DestColor, BlendFactor::Zero),
            blit(dark, 1.0),
            Command::SetBlend(BlendMode::NORMAL),
            blend(BlendFactor::SrcColor, BlendFactor::One),
            rect(0.0, 0.0, 320.0, 240.0, 0, 1.0),
            Command::SetBlend(BlendMode::NORMAL),
            Command::SetTarget(Target::Screen),
            blend(BlendFactor::One, BlendFactor::Zero),
            blit(screen, 2.0),
            Command::SetBlend(BlendMode::NORMAL),
            Command::Present,
            // Drawn after the present: the window doesn't show it.
            rect(0.0, 0.0, 639.0, 479.0, 0xFF_FF_FF, 1.0),
        ];
        renderer.render(&frame);
        let (w, h, rgba) = renderer.read_screen().unwrap();
        assert_eq!((w, h), (640, 480));
        if let Ok(path) = std::env::var("SPELUNKY_SHOT") {
            let mut enc = png::Encoder::new(std::fs::File::create(path).unwrap(), w, h);
            enc.set_color(png::ColorType::Rgba);
            enc.write_header().unwrap().write_image_data(&rgba).unwrap();
        }
        let px = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
        };
        let near = |got: [u8; 4], want: [u8; 4]| {
            assert!(
                got.iter().zip(want).all(|(&g, w)| g.abs_diff(w) <= 1),
                "{got:?} != {want:?}"
            );
        };
        // Lit: the room times the light (127/255 on red and green), opaque.
        near(px(2 * 100, 2 * 120), [0x60, 0x40, 0x40, 255]);
        near(px(2 * 40, 2 * 40), [0x60, 0x40, 0x40, 255]);
        // The translucent instance blended over the room, lit, and opaque again after the alpha fix.
        near(px(2 * 160, 2 * 120), [0x70, 0x60, 0xA0, 255]);
        // Dark everywhere else.
        near(px(2 * 300, 2 * 220), [0, 0, 0, 255]);
        near(px(2 * 160 + 2 * 70, 2 * 120), [0, 0, 0, 255]);
        // Without the alpha fix, the translucent draw leaves screen's alpha at 0.5 * 0.5 + 1 * 0.5 (Direct3D
        // 8 blends alpha with the colour factors), and the `bm_one, bm_zero` blit copies it.
        let fix = blend(BlendFactor::SrcColor, BlendFactor::One);
        let fix = frame.commands.iter().position(|c| *c == fix).unwrap();
        frame.commands.drain(fix..fix + 3);
        renderer.render(&frame);
        let (_, _, unfixed) = renderer.read_screen().unwrap();
        let i = ((2 * 120 * w + 2 * 160) * 4) as usize;
        assert_eq!(unfixed[i..i + 3], rgba[i..i + 3]);
        assert!(unfixed[i + 3].abs_diff(191) <= 1, "{}", unfixed[i + 3]);
    }

    fn positions(pos: f64, size: f64, tile: bool, view_start: f64, end: f64) -> Vec<f64> {
        tile_positions(pos, size, tile, view_start, end).collect()
    }

    #[test]
    fn tiling_starts_at_or_before_the_room_edge() {
        // Offset 10 wraps to -22 for a 32-wide image; copies run until one starts past the end.
        assert_eq!(positions(10.0, 32.0, true, 0.0, 64.0), [-22.0, 10.0, 42.0]);
        assert_eq!(positions(0.0, 32.0, true, 0.0, 64.0), [0.0, 32.0]);
        assert_eq!(positions(10.0, 32.0, false, 0.0, 64.0), [10.0]);
    }

    #[test]
    fn surfaces_ignore_the_port_and_frames_keep_their_start_state() {
        let mut gm = DrawState {
            target: Target::Surface(SurfaceId(1)),
            blend: BlendMode::ADD,
            ..DrawState::initial(640, 480)
        };
        gm.apply(
            &Command::SetView {
                view: Rect::new(32, 16, 320, 240),
                port: Rect::new(0, 0, 640, 480),
            },
            (640, 480),
            |_| Some((320, 240)),
        );
        let state = State::resolve(&gm, 320, 240);
        assert_eq!(state.port, Rect::new(0, 0, 320, 240));
        assert_eq!(state.view, Rect::new(32, 16, 320, 240));
        assert_eq!(state.blend, BlendMode::ADD);
        // Room (32, 16) lands on the surface's first pixel centre (wgpu's half-pixel convention).
        let tf = Transform::new(&state);
        assert_eq!(
            tf.apply(32.0, 16.0),
            [-1.0 + 1.0 / 320.0, 1.0 - 1.0 / 240.0]
        );
        let screen = State::resolve(
            &DrawState {
                target: Target::Screen,
                ..gm
            },
            640,
            480,
        );
        assert_eq!(screen.port, Rect::new(0, 0, 640, 480));
    }

    #[test]
    fn tiling_skips_copies_before_the_view() {
        assert_eq!(
            positions(0.0, 32.0, true, 100.0, 164.0),
            [96.0, 128.0, 160.0]
        );
        assert_eq!(
            tile_positions(0.0, 1e-9, true, 0.0, 320.0).count(),
            MAX_TILE_COPIES as usize
        );
    }
}
