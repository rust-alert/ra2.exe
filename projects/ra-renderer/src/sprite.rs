//! 单张 RGBA 精灵的纹理四边形绘制。

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::rgba_image::RgbaImage;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
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
}

impl SpriteGpu {
    pub fn create(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        image: &RgbaImage,
    ) -> Self {
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
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ra.sprite.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_WGSL.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ra.sprite.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ra.sprite.sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let (texture, bind_group, vertex_buffer) =
            upload(device, queue, &bind_group_layout, &sampler, image);

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            texture,
            bind_group,
            vertex_buffer,
            width: image.width,
            height: image.height,
        }
    }

    pub fn replace_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &RgbaImage,
    ) {
        let (texture, bind_group, vertex_buffer) =
            upload(device, queue, &self.bind_group_layout, &self.sampler, image);
        self.texture = texture;
        self.bind_group = bind_group;
        self.vertex_buffer = vertex_buffer;
        self.width = image.width;
        self.height = image.height;
    }

    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        surface_w: u32,
        surface_h: u32,
    ) {
        let verts = centered_quad(self.width, self.height, surface_w, surface_h);
        // 顶点缓冲在创建时按占位写入，每帧用 queue 写会更好；这里用动态偏移简化：
        // 实际在 Renderer 里用 queue.write_buffer。保留接口，由调用方先 update_vertices。
        let _ = verts;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..6, 0..1);
    }

    pub fn write_vertices(&self, queue: &wgpu::Queue, surface_w: u32, surface_h: u32) {
        let verts = centered_quad(self.width, self.height, surface_w, surface_h);
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
    }
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    image: &RgbaImage,
) -> (wgpu::Texture, wgpu::BindGroup, wgpu::Buffer) {
    let size = wgpu::Extent3d {
        width: image.width,
        height: image.height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ra.sprite.tex"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image.pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * image.width),
            rows_per_image: Some(image.height),
        },
        size,
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ra.sprite.bg"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });

    let placeholder = centered_quad(image.width, image.height, 1024, 768);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ra.sprite.vb"),
        contents: bytemuck::cast_slice(&placeholder),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    (texture, bind_group, vertex_buffer)
}

fn centered_quad(img_w: u32, img_h: u32, surf_w: u32, surf_h: u32) -> [Vertex; 6] {
    let scale = 4.0_f32; // 像素艺术放大，便于看见
    let w = (img_w as f32 * scale) / surf_w.max(1) as f32;
    let h = (img_h as f32 * scale) / surf_h.max(1) as f32;
    let x0 = -w;
    let y0 = -h;
    let x1 = w;
    let y1 = h;
    [
        Vertex {
            pos: [x0, y0],
            uv: [0.0, 1.0],
        },
        Vertex {
            pos: [x1, y0],
            uv: [1.0, 1.0],
        },
        Vertex {
            pos: [x1, y1],
            uv: [1.0, 0.0],
        },
        Vertex {
            pos: [x0, y0],
            uv: [0.0, 1.0],
        },
        Vertex {
            pos: [x1, y1],
            uv: [1.0, 0.0],
        },
        Vertex {
            pos: [x0, y1],
            uv: [0.0, 0.0],
        },
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
