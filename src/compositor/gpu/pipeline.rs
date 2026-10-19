//! 合成管线：uniform 布局、bind group 布局、按输出格式缓存的渲染管线、整数纹理的创建与上传。
use std::collections::HashMap;

/// 着色器源码（编进二进制；它的哈希进判定缓存键，改着色器就会重新评估驱动）
pub const WGSL: &str = include_str!("compose.wgsl");

/// 与 `compose.wgsl` 的 `struct U` 逐字段对应（20 个 4 字节标量 = 80 字节，是 16 的倍数）
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Uniforms {
    pub width: u32,
    pub height: u32,
    pub content_h: u32,
    pub scroll_px: i32,
    pub canvas_w: u32,
    pub canvas_h: u32,
    pub ring_rows: u32,
    pub page_bg: u32,
    pub has_tabbar: u32,
    pub tabbar_on: u32,
    pub tabbar_px: u32,
    pub tabbar_w: u32,
    pub tabbar_h: u32,
    pub fixed_on: u32,
    pub fixed_row0: u32,
    pub fixed_row_end: u32,
    pub fixed_w: u32,
    pub pull_rows: u32,
    pub mode: u32,
    pub fault_tint: u32,
}

const _: () = assert!(std::mem::size_of::<Uniforms>() == 80);

impl Uniforms {
    pub fn bytes(&self) -> &[u8] {
        // SAFETY: #[repr(C)] 的 20 个 u32/i32，没有填充字节（上面的 const 断言守着尺寸）
        unsafe { std::slice::from_raw_parts((self as *const Self).cast::<u8>(), 80) }
    }
}

/// 一张整数纹理（Rgba8Uint）及其视图
pub struct Tex {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub w: u32,
    pub h: u32,
}

impl Tex {
    pub fn new(device: &wgpu::Device, label: &str, w: u32, h: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view, w: w.max(1), h: h.max(1) }
    }

    /// 尺寸不够就重建（只增不减，避免指示器这种逐帧变高的层每帧重建）
    pub fn ensure(slot: &mut Option<Tex>, device: &wgpu::Device, label: &str, w: u32, h: u32) {
        let ok = slot.as_ref().is_some_and(|t| t.w == w.max(1) && t.h >= h.max(1));
        if !ok {
            *slot = Some(Tex::new(device, label, w, h));
        }
    }

    /// 传 `rows` 行（从第 `y0` 行起），源行宽 = `src_w` 像素（每像素 4 字节）
    pub fn upload_rows(&self, queue: &wgpu::Queue, data: &[u8], src_w: u32, y0: u32, rows: u32, w: u32) -> u64 {
        if rows == 0 || w == 0 {
            return 0;
        }
        let offset = (y0 as usize) * (src_w as usize) * 4;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: y0, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &data[offset..],
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(src_w * 4), rows_per_image: None },
            wgpu::Extent3d { width: w, height: rows, depth_or_array_layers: 1 },
        );
        w as u64 * rows as u64 * 4
    }
}

pub struct Pipeline {
    pub layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    by_format: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    pub uniforms: wgpu::Buffer,
    /// 某层不存在时绑的 1×1 占位
    pub ph2d: Tex,
    pub ph_array: wgpu::TextureView,
}

fn tex_entry(binding: u32, dim: wgpu::TextureViewDimension) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Uint,
            view_dimension: dim,
            multisampled: false,
        },
        count: None,
    }
}

impl Pipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compose.wgsl"),
            source: wgpu::ShaderSource::Wgsl(WGSL.into()),
        });
        let d2 = wgpu::TextureViewDimension::D2;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("合成 bind group"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                tex_entry(1, wgpu::TextureViewDimension::D2Array),
                tex_entry(2, d2),
                tex_entry(3, d2),
                tex_entry(4, d2),
                tex_entry(5, d2),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("合成"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("合成 uniform"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ph2d = Tex::new(device, "占位", 1, 1);
        let ph_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("占位 array"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let ph_array = ph_tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        Self { layout, pipeline_layout, shader, by_format: HashMap::new(), uniforms, ph2d, ph_array }
    }

    /// 输出格式对应的管线（第一次用到时建，之后缓存）
    pub fn for_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) -> &wgpu::RenderPipeline {
        let (shader, layout) = (&self.shader, &self.pipeline_layout);
        self.by_format.entry(format).or_insert_with(|| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("合成"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    // blend: None —— 硬件混合是浮点舍入，会差 1 LSB；混合全在着色器里用整数做
                    targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        })
    }
}
