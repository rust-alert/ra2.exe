//! 单张 RGBA 精灵的纹理四边形绘制。
//!
//! **过渡用途**：预览底图等单图显示。完整内容运行应迁到 atlas / array + instance buffer，
//! 禁止把「每对象一张纹理 + `replace_image`」扩展为长期单位/UI 模型。

use bytemuck::{Pod, Zeroable};
use image::RgbaImage;
use wgpu::util::DeviceExt;

use crate::camera::Camera;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

/// 精灵采样 / 写出的色域语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteColorSpace {
    /// 纹理按 sRGB 上传，目标按表面 sRGB 语义（地图预览等）。
    Srgb,
    /// 编码字节直通：`Rgba8Unorm` 纹理，写入表面的 unorm 视图（壳层 UI）。
    ///
    /// 避免把已经是显示域的字节再当线性色做一次 sRGB 编码（观感发白）。
    EncodedBytes,
}

pub struct SpriteGpu {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    color_space: SpriteColorSpace,
    /// 创建 pipeline 时的颜色目标格式（sRGB 表面或 unorm 别名）。
    target_format: wgpu::TextureFormat,
}

impl SpriteGpu {
    /// 按色域语义创建精灵（预览用 [`SpriteColorSpace::Srgb`]）。
    pub fn create(device: &wgpu::Device, queue: &wgpu::Queue, surface_format: wgpu::TextureFormat, image: &RgbaImage) -> Self {
        Self::create_with_color_space(device, queue, surface_format, image, SpriteColorSpace::Srgb)
    }

    /// 创建精灵并指定采样/写出色域。
    pub fn create_with_color_space(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        image: &RgbaImage,
        color_space: SpriteColorSpace,
    ) -> Self {
        let target_format = match color_space {
            SpriteColorSpace::Srgb => surface_format,
            SpriteColorSpace::EncodedBytes => surface_format.remove_srgb_suffix(),
        };
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ra.sprite.bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ra.sprite.pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ra.sprite.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_WGSL.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(match color_space {
                SpriteColorSpace::Srgb => "ra.sprite.pipeline.srgb",
                SpriteColorSpace::EncodedBytes => "ra.sprite.pipeline.encoded",
            }),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ra.sprite.sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let (texture, bind_group, vertex_buffer) = upload(device, queue, &bind_group_layout, &sampler, image, color_space);

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            texture,
            bind_group,
            vertex_buffer,
            width: image.width(),
            height: image.height(),
            color_space,
            target_format,
        }
    }

    pub fn replace_image(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, image: &RgbaImage) {
        let (texture, bind_group, vertex_buffer) =
            upload(device, queue, &self.bind_group_layout, &self.sampler, image, self.color_space);
        self.texture = texture;
        self.bind_group = bind_group;
        self.vertex_buffer = vertex_buffer;
        self.width = image.width();
        self.height = image.height();
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// 该精灵写出时应使用的颜色目标格式（可能是表面 sRGB，或 unorm 别名）。
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    /// 是否为编码字节直通壳层路径。
    pub fn is_encoded_bytes(&self) -> bool {
        self.color_space == SpriteColorSpace::EncodedBytes
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..6, 0..1);
    }

    pub fn write_vertices(&self, queue: &wgpu::Queue, camera: &Camera, surface_w: u32, surface_h: u32) {
        let verts = camera_quad(self.width, self.height, camera, surface_w, surface_h);
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
    }
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    image: &RgbaImage,
    color_space: SpriteColorSpace,
) -> (wgpu::Texture, wgpu::BindGroup, wgpu::Buffer) {
    let size = wgpu::Extent3d { width: image.width(), height: image.height(), depth_or_array_layers: 1 };
    let format = match color_space {
        SpriteColorSpace::Srgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        SpriteColorSpace::EncodedBytes => wgpu::TextureFormat::Rgba8Unorm,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(match color_space {
            SpriteColorSpace::Srgb => "ra.sprite.tex.srgb",
            SpriteColorSpace::EncodedBytes => "ra.sprite.tex.encoded",
        }),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        image.as_raw(),
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * image.width()), rows_per_image: Some(image.height()) },
        size,
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ra.sprite.bg"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    });

    let cam = Camera::fit(image.width(), image.height(), 1024, 768);
    let placeholder = camera_quad(image.width(), image.height(), &cam, 1024, 768);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ra.sprite.vb"),
        contents: bytemuck::cast_slice(&placeholder),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    (texture, bind_group, vertex_buffer)
}

fn camera_quad(img_w: u32, img_h: u32, camera: &Camera, surf_w: u32, surf_h: u32) -> [Vertex; 6] {
    let sw = surf_w.max(1) as f32;
    let sh = surf_h.max(1) as f32;
    let w = img_w as f32;
    let h = img_h as f32;
    let p00 = camera.world_to_ndc(0.0, 0.0, sw, sh);
    let p10 = camera.world_to_ndc(w, 0.0, sw, sh);
    let p11 = camera.world_to_ndc(w, h, sw, sh);
    let p01 = camera.world_to_ndc(0.0, h, sw, sh);
    [
        Vertex { pos: p00, uv: [0.0, 0.0] },
        Vertex { pos: p10, uv: [1.0, 0.0] },
        Vertex { pos: p11, uv: [1.0, 1.0] },
        Vertex { pos: p00, uv: [0.0, 0.0] },
        Vertex { pos: p11, uv: [1.0, 1.0] },
        Vertex { pos: p01, uv: [0.0, 1.0] },
    ]
}

const SPRITE_WGSL: &str = r#"
struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(v: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = vec4<f32>(v.pos, 0.0, 1.0);
    out.uv = v.uv;
    return out;
}

@group(0) @binding(0) var sprite_tex: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(sprite_tex, sprite_sampler, in.uv);
}
"#;
