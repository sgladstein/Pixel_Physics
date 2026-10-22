//! **The interface layer's half on the GPU**: one texture the size of
//! [`LAYER_SIZE`] and one pass that draws the box and the layer over it into
//! the window, in place of the `pixels` crate's own scaling pass.
//!
//! `Lab::draw_layered` paints the interface into a CPU buffer at twice the
//! logical resolution, premultiplied and transparent where the box shows,
//! and says whether it repainted. This uploads that buffer only when it did,
//! and composites every frame.
//!
//! **Why the box's texture is `Rgba8Unorm` rather than the crate's default
//! sRGB one.** Everything the CPU blends -- BIOSPHERE's 0.82 panel, the help
//! page's 0.75 dim -- is blended in gamma space, on the bytes. The GPU's
//! blend hardware works in linear space on an sRGB target, so doing the
//! "over" there would make every translucent panel read lighter on screen
//! than in a screenshot. So the composite is done in the shader on the raw
//! bytes, `layer + box * (1 - layer alpha)`, exactly what
//! `render::composite_layer` does for the screenshot, and only the result is
//! decoded for an sRGB surface. With an opaque layer pixel or no layer at
//! all this is the identity, as the crate's own pass was.
//!
//! **Sampling.** The box texel under each window pixel is fetched directly
//! (nearest, as the crate's pixel-perfect pass does). The layer is fetched
//! directly too when the window holds it a whole number of times -- the
//! default 1024x640 window is exactly 1:1 -- and sampled bilinearly otherwise,
//! because nearest at 1.5x on a high-DPI screen doubles every third column of
//! a glyph.

use pixel_physics::lab::LAYER_SIZE;
use pixels::wgpu;
use pixels::Pixels;

const SHADER: &str = r#"
struct U {
    clip: vec4<f32>,
    flags: vec4<f32>,
};
@group(0) @binding(0) var world: texture_2d<f32>;
@group(0) @binding(1) var ui: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;
@group(0) @binding(3) var<uniform> u: U;

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(i & 1u) * 4 - 1);
    let y = f32(i32(i >> 1u) * 4 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn decode(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = (p.xy - u.clip.xy) / u.clip.zw;
    let wd = vec2<f32>(textureDimensions(world));
    let w = textureLoad(world, vec2<i32>(clamp(floor(uv * wd), vec2<f32>(0.0), wd - 1.0)), 0);
    let ld = vec2<f32>(textureDimensions(ui));
    var l = textureSampleLevel(ui, smp, uv, 0.0);
    if (u.flags.y < 0.5) {
        l = textureLoad(ui, vec2<i32>(clamp(floor(uv * ld), vec2<f32>(0.0), ld - 1.0)), 0);
    }
    var rgb = l.rgb + w.rgb * (1.0 - l.a);
    if (u.flags.x > 0.5) {
        rgb = decode(rgb);
    }
    return vec4<f32>(rgb, 1.0);
}
"#;

pub struct Layer {
    /// What `Lab::draw_layered` paints into.
    pub cpu: Vec<u8>,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    /// Bound to the box texture of this size; `pixels` replaces that texture
    /// when the buffer is resized, so a new size means a new bind group.
    bound: Option<((u32, u32), wgpu::BindGroup)>,
    srgb: bool,
}

impl Layer {
    pub fn new(pixels: &Pixels<'static>) -> Self {
        let device = pixels.device();
        let (w, h) = LAYER_SIZE;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("lab_ui_layer"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lab_ui_layer_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lab_ui_layer_uniform"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lab_ui_layer_bind_group_layout"),
            entries: &[
                texture_entry(0),
                texture_entry(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lab_ui_layer_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lab_ui_layer_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let format = pixels.render_texture_format();
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lab_ui_layer_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Layer {
            cpu: vec![0; (w * h * 4) as usize],
            texture,
            view,
            sampler,
            uniform,
            layout,
            pipeline,
            bound: None,
            srgb: format.is_srgb(),
        }
    }

    /// Copy the CPU layer to its texture. Only after a repaint.
    pub fn upload(&self, pixels: &Pixels<'static>) {
        let (w, h) = LAYER_SIZE;
        pixels.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.cpu,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }

    /// The box and the layer over it, into the window: `pixels.render_with`
    /// with this pass instead of the crate's scaling pass.
    pub fn render(&mut self, pixels: &Pixels<'static>) -> Result<(), pixels::Error> {
        let world = pixels.texture();
        let size = (world.width(), world.height());
        if self.bound.as_ref().is_none_or(|(s, _)| *s != size) {
            let world_view = world.create_view(&wgpu::TextureViewDescriptor::default());
            let group = pixels.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lab_ui_layer_bind_group"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&world_view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 3, resource: self.uniform.as_entire_binding() },
                ],
            });
            self.bound = Some((size, group));
        }
        let (cx, cy, cw, ch) = pixels.context().scaling_renderer.clip_rect();
        // Fetched directly when the window holds the layer a whole number of
        // times over, bilinear otherwise -- see the module doc.
        let whole = cw >= LAYER_SIZE.0 && cw % LAYER_SIZE.0 == 0 && ch % LAYER_SIZE.1 == 0;
        let u: [f32; 8] = [
            cx as f32,
            cy as f32,
            cw.max(1) as f32,
            ch.max(1) as f32,
            if self.srgb { 1.0 } else { 0.0 },
            if whole { 0.0 } else { 1.0 },
            0.0,
            0.0,
        ];
        let bytes: Vec<u8> = u.iter().flat_map(|v| v.to_le_bytes()).collect();
        pixels.queue().write_buffer(&self.uniform, 0, &bytes);
        let group = &self.bound.as_ref().expect("bound above").1;
        let pipeline = &self.pipeline;
        pixels.render_with(|encoder, target, _context| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lab_ui_layer_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.set_scissor_rect(cx, cy, cw, ch);
            pass.draw(0..3, 0..1);
            Ok(())
        })
    }
}
