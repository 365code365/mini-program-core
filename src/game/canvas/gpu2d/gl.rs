//! GPU 2D 的着色器与批处理：一个程序、一种顶点格式，三种片元（v_k）：
//!
//! - 0 = 轴对齐抗锯齿矩形（fillRect 的热路径）：覆盖率在片元里按 CPU 光栅器（canvas/rect_aa.rs）同一个算法算 ——
//!   横向按像素与矩形的重叠长度，纵向数 4 条子扫描线里命中几条，`round(a × 覆盖率)` 取整。所以同一个矩形两边逐像素
//!   差的只是混合的舍入（CPU 存非预乘、GPU 存预乘）。
//! - 1 = 非预乘纹理（图片、CPU 画好的临时层）：片元里预乘，再乘 globalAlpha；uv 落在 [0,1] 外给 0
//!   （「会清掉形状外像素」的合成模式铺满整块画布画，框外就是源为 0）。
//! - 2 = 预乘纹理（另一块 GPU 画布当源）。
//! - 3 = CPU 画好的临时层（非预乘），纹理只用左上角一块（纹理只增不缩，texSubImage 更新：每帧重新分配存储在
//!   ANGLE / Metal 上一次就要几百微秒）。
//!
//! 裁剪：CPU 的做法是「画之前拍快照、画完按蒙版在前后之间插值」，对不清形状外像素的合成模式，它恰好等于
//! 源 × 蒙版（结果对源是线性的），所以 GPU 上采样裁剪蒙版乘到源上。清形状外像素的模式 + 裁剪走 CPU。
//!
//! 坐标：画布行 0 放在 GL 的 y = 0（帧缓冲的底），纹理行 0 = 画布顶行，读回、上传都不用翻；只有上屏到窗口时翻一次。

use crate::renderer::components::GpuBlend;
use glow::HasContext;

/// 每个顶点：位置 2、uv 2、颜色 4（非预乘，a = 不透明度）、矩形 4（l r t b）、种类 1
pub const FLOATS: usize = 13;

const VS: &str = "attribute vec2 a_p; attribute vec2 a_uv; attribute vec4 a_c; attribute vec4 a_r; attribute float a_k;
uniform vec2 u_vp;
varying vec2 v_uv; varying vec4 v_c; varying vec4 v_r; varying float v_k;
void main(){ v_uv = a_uv; v_c = a_c; v_r = a_r; v_k = a_k; gl_Position = vec4(a_p.x / u_vp.x * 2.0 - 1.0, a_p.y / u_vp.y * 2.0 - 1.0, 0.0, 1.0); }";

const FS: &str = "precision highp float;
uniform sampler2D u_t; uniform sampler2D u_clip; uniform float u_hasclip; uniform vec2 u_vp;
varying vec2 v_uv; varying vec4 v_c; varying vec4 v_r; varying float v_k;
void main(){
  vec4 o;
  if (v_k < 0.5) {
    vec2 px = floor(gl_FragCoord.xy);
    float ox = clamp(min(px.x + 1.0, v_r.y) - max(px.x, v_r.x), 0.0, 1.0);
    float k = 0.0;
    for (int s = 0; s < 4; s++) { float sy = px.y + (float(s) + 0.5) / 4.0; if (sy >= v_r.z && sy < v_r.w) k += 1.0; }
    float a = floor(v_c.a * 255.0 * ox * k / 4.0 + 0.5) / 255.0;
    o = vec4(v_c.rgb * a, a);
  } else {
    // 图片给了源矩形（内缩半个纹素）：夹在里面采样，精灵图集相邻的格子不渗进来（HTML drawImage 的严格源矩形）
    // 临时层（v_k = 3）：纹理只用了左上角 (v_r.y, v_r.w) 那块，落在块外（含清形状外像素的模式铺满画布时）给 0
    vec2 uv = v_k < 2.5 && v_r.y > v_r.x ? clamp(v_uv, v_r.xz, v_r.yw) : v_uv;
    vec2 hi = v_k > 2.5 ? v_r.yw : vec2(1.0);
    vec4 t = texture2D(u_t, uv);
    if (v_uv.x < 0.0 || v_uv.y < 0.0 || v_uv.x > hi.x || v_uv.y > hi.y) t = vec4(0.0);
    o = (v_k < 1.5 || v_k > 2.5 ? vec4(t.rgb * t.a, t.a) : t) * v_c.a;
  }
  if (u_hasclip > 0.5) o *= texture2D(u_clip, gl_FragCoord.xy / u_vp).a;
  gl_FragColor = o;
}";

/// 上屏：整块纹理（预乘）铺到窗口，翻 y；`u_op` = 1 时 alpha 写 1（主画布压在黑底上，与 CPU 的 to_rgba_opaque 一致）
const VS_BLIT: &str = "attribute vec2 a_p; uniform float u_flip; varying vec2 v_uv;
void main(){ v_uv = vec2(a_p.x * 0.5 + 0.5, mix(a_p.y * 0.5 + 0.5, 0.5 - a_p.y * 0.5, u_flip)); gl_Position = vec4(a_p, 0.0, 1.0); }";
const FS_BLIT: &str = "precision mediump float; varying vec2 v_uv; uniform sampler2D u_t; uniform float u_op;
void main(){ vec4 t = texture2D(u_t, v_uv); gl_FragColor = u_op > 0.5 ? vec4(t.rgb, 1.0) : t; }";

pub struct Prog {
    pub prog: glow::NativeProgram,
    pub u_vp: Option<glow::NativeUniformLocation>,
    pub u_hasclip: Option<glow::NativeUniformLocation>,
    pub vbo: glow::NativeBuffer,
    pub blit: glow::NativeProgram,
    pub blit_flip: Option<glow::NativeUniformLocation>,
    pub blit_op: Option<glow::NativeUniformLocation>,
    pub quad: glow::NativeBuffer,
}

unsafe fn shader(gl: &glow::Context, t: u32, src: &str) -> Result<glow::NativeShader, String> {
    let s = gl.create_shader(t)?;
    gl.shader_source(s, src);
    gl.compile_shader(s);
    if gl.get_shader_compile_status(s) { Ok(s) } else { Err(gl.get_shader_info_log(s)) }
}

unsafe fn link(gl: &glow::Context, vs: &str, fs: &str, attrs: &[&str]) -> Result<glow::NativeProgram, String> {
    let p = gl.create_program()?;
    gl.attach_shader(p, shader(gl, glow::VERTEX_SHADER, vs)?);
    gl.attach_shader(p, shader(gl, glow::FRAGMENT_SHADER, fs)?);
    for (i, a) in attrs.iter().enumerate() {
        gl.bind_attrib_location(p, i as u32, a);
    }
    gl.link_program(p);
    if gl.get_program_link_status(p) { Ok(p) } else { Err(gl.get_program_info_log(p)) }
}

impl Prog {
    pub unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        let prog = link(gl, VS, FS, &["a_p", "a_uv", "a_c", "a_r", "a_k"])?;
        gl.use_program(Some(prog));
        if let Some(l) = gl.get_uniform_location(prog, "u_t") {
            gl.uniform_1_i32(Some(&l), 0);
        }
        if let Some(l) = gl.get_uniform_location(prog, "u_clip") {
            gl.uniform_1_i32(Some(&l), 1);
        }
        let blit = link(gl, VS_BLIT, FS_BLIT, &["a_p"])?;
        let quad = gl.create_buffer()?;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad));
        let q: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&q), glow::STATIC_DRAW);
        Ok(Self {
            u_vp: gl.get_uniform_location(prog, "u_vp"),
            u_hasclip: gl.get_uniform_location(prog, "u_hasclip"),
            vbo: gl.create_buffer()?,
            blit_flip: gl.get_uniform_location(blit, "u_flip"),
            blit_op: gl.get_uniform_location(blit, "u_op"),
            prog,
            blit,
            quad,
        })
    }

    /// 画一批顶点（调用方已经绑好帧缓冲、纹理、混合）
    pub unsafe fn draw(&self, gl: &glow::Context, verts: &[f32], vp: (u32, u32), clip: bool) {
        if verts.is_empty() {
            return;
        }
        gl.use_program(Some(self.prog));
        gl.uniform_2_f32(self.u_vp.as_ref(), vp.0 as f32, vp.1 as f32);
        gl.uniform_1_f32(self.u_hasclip.as_ref(), if clip { 1.0 } else { 0.0 });
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(verts), glow::STREAM_DRAW);
        let stride = (FLOATS * 4) as i32;
        for (i, (n, off)) in [(2, 0), (2, 2), (4, 4), (4, 8), (1, 12)].into_iter().enumerate() {
            gl.enable_vertex_attrib_array(i as u32);
            gl.vertex_attrib_pointer_f32(i as u32, n, glow::FLOAT, false, stride, off * 4);
        }
        gl.draw_arrays(glow::TRIANGLES, 0, (verts.len() / FLOATS) as i32);
    }

    /// 整块纹理铺满当前视口（上屏）
    pub unsafe fn blit(&self, gl: &glow::Context, flip: bool, opaque: bool) {
        gl.use_program(Some(self.blit));
        gl.uniform_1_f32(self.blit_flip.as_ref(), if flip { 1.0 } else { 0.0 });
        gl.uniform_1_f32(self.blit_op.as_ref(), if opaque { 1.0 } else { 0.0 });
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.quad));
        for i in 1..5 {
            gl.disable_vertex_attrib_array(i);
        }
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}

pub fn bytes(v: &[f32]) -> &[u8] {
    // SAFETY: f32 切片按字节看，长度 ×4，对齐只会变宽松
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

/// 预乘源（rgb 已乘 a）的 Porter-Duff 两个系数（结果 = 源 × fa + 目标 × fb），与 fx_composite.rs 的表一一对应
pub fn blend_func(b: GpuBlend) -> (u32, u32) {
    use glow::*;
    match b {
        GpuBlend::SourceOver => (ONE, ONE_MINUS_SRC_ALPHA),
        GpuBlend::SourceIn => (DST_ALPHA, ZERO),
        GpuBlend::SourceOut => (ONE_MINUS_DST_ALPHA, ZERO),
        GpuBlend::SourceAtop => (DST_ALPHA, ONE_MINUS_SRC_ALPHA),
        GpuBlend::DestinationOver => (ONE_MINUS_DST_ALPHA, ONE),
        GpuBlend::DestinationIn => (ZERO, SRC_ALPHA),
        GpuBlend::DestinationOut => (ZERO, ONE_MINUS_SRC_ALPHA),
        GpuBlend::DestinationAtop => (ONE_MINUS_DST_ALPHA, SRC_ALPHA),
        GpuBlend::Lighter => (ONE, ONE),
        GpuBlend::Copy => (ONE, ZERO),
        GpuBlend::Xor => (ONE_MINUS_DST_ALPHA, ONE_MINUS_SRC_ALPHA),
    }
}

/// 一个四边形的六个顶点（两个三角形）：`p` 四角按 0-1-2-3 绕一圈，`uv` 同序
pub fn push_quad(v: &mut Vec<f32>, p: [(f32, f32); 4], uv: [(f32, f32); 4], c: [f32; 4], r: [f32; 4], k: f32) {
    for i in [0usize, 1, 2, 0, 2, 3] {
        v.extend_from_slice(&[p[i].0, p[i].1, uv[i].0, uv[i].1, c[0], c[1], c[2], c[3], r[0], r[1], r[2], r[3], k]);
    }
}

/// 新建一张纹理（不分配存储），给定过滤方式、边缘夹紧
pub unsafe fn new_texture(gl: &glow::Context, filter: u32) -> Result<glow::NativeTexture, String> {
    let t = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(t));
    set_filter(gl, filter);
    for p in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
        gl.tex_parameter_i32(glow::TEXTURE_2D, p, glow::CLAMP_TO_EDGE as i32);
    }
    Ok(t)
}

pub unsafe fn set_filter(gl: &glow::Context, filter: u32) {
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter as i32);
}

/// RGBA8 传进当前绑定的纹理（整张）
pub unsafe fn upload_rgba(gl: &glow::Context, w: u32, h: u32, px: &[u8]) {
    gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
    gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA as i32, w as i32, h as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(px)));
}
