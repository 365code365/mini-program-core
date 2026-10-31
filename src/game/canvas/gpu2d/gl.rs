//! GPU 2D 的着色器与批处理：一个程序、一种顶点格式、带索引的四边形（每个 4 个顶点），片元按种类（v_k）分：
//!
//! - 0 = 轴对齐抗锯齿矩形（fillRect 的热路径）：覆盖率在片元里按 CPU 光栅器（canvas/rect_aa.rs）同一个算法算 ——
//!   横向按像素与矩形的重叠长度，纵向数 4 条子扫描线里命中几条，`round(a × 覆盖率)` 取整。所以同一个矩形两边逐像素
//!   差的只是混合的舍入（CPU 存非预乘、GPU 存预乘）。
//! - 1 = 非预乘纹理（图片 / CPU 画布当源），2 = 预乘纹理（另一块 GPU 画布当源）：uv 夹在内缩半纹素的源矩形里
//!   （精灵图集相邻的格子不渗进来），再乘 globalAlpha。
//! - 3 = CPU 画好的临时层（非预乘），纹理只用左上角一块（纹理只增不缩，texSubImage 更新：每帧重新分配存储在
//!   ANGLE / Metal 上一次就要几百微秒）；落在块外（清形状外像素的模式铺满画布时）给 0。
//! - 4 = 同 0 的矩形覆盖率 × 渐变，5 = 临时层的 alpha（CPU 用白色画的覆盖率）× 渐变。渐变在片元里按
//!   `fx2d::DeviceSource::sample` 同一个式子算（线性 / 径向的同心近似 / 锥形，色标线性插值，最多 [`MAX_STOPS`] 个）。
//!
//! 边缘抗锯齿（旋转 / 斜切的 drawImage）：每个顶点带到四条边的设备距离 v_e（仿射下线性插值是精确的），
//! 覆盖率 = Π clamp(v_e + 0.5, 0, 1)，与 CPU 的 image2d::sample_affine 同一个式子；不需要的给一个大数（= 1）。
//!
//! 裁剪：CPU 的做法是「画之前拍快照、画完按蒙版在前后之间插值」，对不清形状外像素的合成模式，它恰好等于
//! 源 × 蒙版（结果对源是线性的），所以 GPU 上采样裁剪蒙版乘到源上。清形状外像素的模式 + 裁剪走 CPU。
//!
//! 坐标：画布行 0 放在 GL 的 y = 0（帧缓冲的底），纹理行 0 = 画布顶行，读回、上传都不用翻；只有上屏到窗口时翻一次。

use crate::renderer::components::GpuBlend;
use glow::HasContext;

/// 每个顶点：位置 2、uv 2、颜色 4（非预乘，a = 不透明度）、矩形 4（l r t b / 源矩形）、边距 4、种类 1
pub const FLOATS: usize = 17;
/// 一批最多几个四边形（u16 索引：4 × 16384 = 65536 个顶点）
pub const MAX_QUADS: usize = 16384;
/// 渐变色标上限（再多走 CPU）
pub const MAX_STOPS: usize = 8;
/// 「不做边缘抗锯齿」的边距
pub const NO_EDGE: [[f32; 4]; 4] = [[1e5; 4]; 4];

const VS: &str = "attribute vec2 a_p; attribute vec2 a_uv; attribute vec4 a_c; attribute vec4 a_r; attribute vec4 a_e; attribute float a_k;
uniform vec2 u_vp;
varying vec2 v_uv; varying vec4 v_c; varying vec4 v_r; varying vec4 v_e; varying float v_k;
void main(){ v_uv = a_uv; v_c = a_c; v_r = a_r; v_e = a_e; v_k = a_k; gl_Position = vec4(a_p.x / u_vp.x * 2.0 - 1.0, a_p.y / u_vp.y * 2.0 - 1.0, 0.0, 1.0); }";

const FS: &str = "precision highp float;
uniform sampler2D u_t; uniform sampler2D u_clip; uniform float u_hasclip; uniform vec2 u_vp;
uniform int u_gk; uniform vec4 u_g0; uniform float u_g1; uniform int u_gn; uniform float u_gp[8]; uniform vec4 u_gc[8];
varying vec2 v_uv; varying vec4 v_c; varying vec4 v_r; varying vec4 v_e; varying float v_k;
vec4 grad(vec2 p){
  float t = 0.0;
  if (u_gk == 0) { if (u_g1 > 0.0) t = dot(p - u_g0.xy, u_g0.zw) / u_g1; }
  else if (u_gk == 1) { if (abs(u_g0.w - u_g0.z) > 1e-4) t = (length(p - u_g0.xy) - u_g0.z) / (u_g0.w - u_g0.z); }
  else { float a = atan(p.y - u_g0.y, p.x - u_g0.x) - u_g0.z; t = mod(a, 6.28318530718) / 6.28318530718; }
  if (t <= u_gp[0]) return u_gc[0];
  vec4 c = u_gc[0];
  for (int i = 1; i < 8; i++) {
    if (i >= u_gn) break;
    if (t <= u_gp[i]) { float k = u_gp[i] > u_gp[i - 1] ? (t - u_gp[i - 1]) / (u_gp[i] - u_gp[i - 1]) : 1.0; return mix(u_gc[i - 1], u_gc[i], k); }
    c = u_gc[i];
  }
  return c;
}
float rectcov(){
  vec2 px = floor(gl_FragCoord.xy);
  float ox = clamp(min(px.x + 1.0, v_r.y) - max(px.x, v_r.x), 0.0, 1.0);
  float k = 0.0;
  for (int s = 0; s < 4; s++) { float sy = px.y + (float(s) + 0.5) / 4.0; if (sy >= v_r.z && sy < v_r.w) k += 1.0; }
  return floor(255.0 * ox * k / 4.0 + 0.5) / 255.0;
}
void main(){
  vec4 o;
  if (v_k < 0.5) {
    vec2 px = floor(gl_FragCoord.xy);
    float ox = clamp(min(px.x + 1.0, v_r.y) - max(px.x, v_r.x), 0.0, 1.0);
    float k = 0.0;
    for (int s = 0; s < 4; s++) { float sy = px.y + (float(s) + 0.5) / 4.0; if (sy >= v_r.z && sy < v_r.w) k += 1.0; }
    float a = floor(v_c.a * 255.0 * ox * k / 4.0 + 0.5) / 255.0;
    o = vec4(v_c.rgb * a, a);
  } else if (v_k < 2.5) {
    vec4 t = texture2D(u_t, clamp(v_uv, v_r.xz, v_r.yw));
    o = (v_k < 1.5 ? vec4(t.rgb * t.a, t.a) : t) * v_c.a;
  } else if (v_k < 3.5 || v_k > 4.5) {
    vec4 t = texture2D(u_t, v_uv);
    if (v_uv.x < 0.0 || v_uv.y < 0.0 || v_uv.x > v_r.y || v_uv.y > v_r.w) t = vec4(0.0);
    if (v_k < 3.5) { o = vec4(t.rgb * t.a, t.a); }
    else { vec4 g = grad(gl_FragCoord.xy); float a = g.a * t.a * v_c.a; o = vec4(g.rgb * a, a); }
  } else {
    vec4 g = grad(gl_FragCoord.xy); float a = g.a * rectcov() * v_c.a; o = vec4(g.rgb * a, a);
  }
  vec4 ec = clamp(v_e + 0.5, 0.0, 1.0);
  o *= ec.x * ec.y * ec.z * ec.w;
  if (u_hasclip > 0.5) o *= texture2D(u_clip, gl_FragCoord.xy / u_vp).a;
  gl_FragColor = o;
}";

/// 整块纹理铺满视口：上屏（翻 y，`u_op` = 1 时 alpha 写 1：主画布压在黑底上，与 CPU 的 to_rgba_opaque 一致），
/// 或拷进 WebGL 的纹理（`u_op` = 2 去预乘：WebGL 默认要非预乘）
const VS_BLIT: &str = "attribute vec2 a_p; uniform float u_flip; varying vec2 v_uv;
void main(){ v_uv = vec2(a_p.x * 0.5 + 0.5, mix(a_p.y * 0.5 + 0.5, 0.5 - a_p.y * 0.5, u_flip)); gl_Position = vec4(a_p, 0.0, 1.0); }";
const FS_BLIT: &str = "precision highp float; varying vec2 v_uv; uniform sampler2D u_t; uniform float u_op;
void main(){ vec4 t = texture2D(u_t, v_uv);
  if (u_op > 1.5) gl_FragColor = t.a > 0.0 ? vec4(t.rgb / t.a, t.a) : vec4(0.0);
  else gl_FragColor = u_op > 0.5 ? vec4(t.rgb, 1.0) : t; }";

/// 设备坐标里的渐变（`fx2d::DeviceSource` 的 GPU 版，见 gpu_view.rs）
pub use crate::renderer::components::GpuGradient as Grad;

pub struct Prog {
    pub prog: glow::NativeProgram,
    u_vp: Option<glow::NativeUniformLocation>,
    u_hasclip: Option<glow::NativeUniformLocation>,
    u_g: [Option<glow::NativeUniformLocation>; 6],
    pub vbo: glow::NativeBuffer,
    ibo: glow::NativeBuffer,
    pub blit: glow::NativeProgram,
    blit_flip: Option<glow::NativeUniformLocation>,
    blit_op: Option<glow::NativeUniformLocation>,
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
        let prog = link(gl, VS, FS, &["a_p", "a_uv", "a_c", "a_r", "a_e", "a_k"])?;
        gl.use_program(Some(prog));
        let u = |n: &str| gl.get_uniform_location(prog, n);
        if let Some(l) = u("u_t") {
            gl.uniform_1_i32(Some(&l), 0);
        }
        if let Some(l) = u("u_clip") {
            gl.uniform_1_i32(Some(&l), 1);
        }
        let blit = link(gl, VS_BLIT, FS_BLIT, &["a_p"])?;
        let quad = gl.create_buffer()?;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad));
        let q: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&q), glow::STATIC_DRAW);
        // 索引：每个四边形 0-1-2、0-2-3（GLES2 没有 VAO，画之前绑上）
        let idx: Vec<u16> = (0..MAX_QUADS as u16).flat_map(|i| { let b = i.wrapping_mul(4); [b, b + 1, b + 2, b, b + 2, b + 3] }).collect();
        let ibo = gl.create_buffer()?;
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, std::slice::from_raw_parts(idx.as_ptr() as *const u8, idx.len() * 2), glow::STATIC_DRAW);
        Ok(Self {
            u_vp: u("u_vp"),
            u_hasclip: u("u_hasclip"),
            u_g: [u("u_gk"), u("u_g0"), u("u_g1"), u("u_gn"), u("u_gp"), u("u_gc")],
            vbo: gl.create_buffer()?,
            ibo,
            blit_flip: gl.get_uniform_location(blit, "u_flip"),
            blit_op: gl.get_uniform_location(blit, "u_op"),
            prog,
            blit,
            quad,
        })
    }

    /// 渐变 uniform（种类 4 / 5 用）
    pub unsafe fn set_grad(&self, gl: &glow::Context, g: &Grad) {
        gl.use_program(Some(self.prog));
        let [k, g0, g1, n, gp, gc] = &self.u_g;
        gl.uniform_1_i32(k.as_ref(), g.kind);
        gl.uniform_4_f32(g0.as_ref(), g.g0[0], g.g0[1], g.g0[2], g.g0[3]);
        gl.uniform_1_f32(g1.as_ref(), g.g1);
        gl.uniform_1_i32(n.as_ref(), g.stops.len() as i32);
        let mut pos = [0f32; MAX_STOPS];
        let mut col = [0f32; MAX_STOPS * 4];
        for (i, (p, c)) in g.stops.iter().take(MAX_STOPS).enumerate() {
            pos[i] = *p;
            col[i * 4..i * 4 + 4].copy_from_slice(c);
        }
        gl.uniform_1_f32_slice(gp.as_ref(), &pos);
        gl.uniform_4_f32_slice(gc.as_ref(), &col);
    }

    /// 画一批四边形（调用方已经绑好帧缓冲、纹理、混合）
    pub unsafe fn draw(&self, gl: &glow::Context, verts: &[f32], vp: (u32, u32), clip: bool) {
        let quads = verts.len() / (FLOATS * 4);
        if quads == 0 {
            return;
        }
        gl.use_program(Some(self.prog));
        gl.uniform_2_f32(self.u_vp.as_ref(), vp.0 as f32, vp.1 as f32);
        gl.uniform_1_f32(self.u_hasclip.as_ref(), if clip { 1.0 } else { 0.0 });
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(verts), glow::STREAM_DRAW);
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));
        let stride = (FLOATS * 4) as i32;
        for (i, (n, off)) in [(2, 0), (2, 2), (4, 4), (4, 8), (4, 12), (1, 16)].into_iter().enumerate() {
            gl.enable_vertex_attrib_array(i as u32);
            gl.vertex_attrib_pointer_f32(i as u32, n, glow::FLOAT, false, stride, off * 4);
        }
        gl.draw_elements(glow::TRIANGLES, (quads * 6) as i32, glow::UNSIGNED_SHORT, 0);
    }

    /// 整块纹理铺满当前视口。`op`：0 原样，1 不透明（上屏），2 去预乘（拷给 WebGL）
    pub unsafe fn blit(&self, gl: &glow::Context, flip: bool, op: u8) {
        gl.use_program(Some(self.blit));
        gl.uniform_1_f32(self.blit_flip.as_ref(), if flip { 1.0 } else { 0.0 });
        gl.uniform_1_f32(self.blit_op.as_ref(), op as f32);
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.quad));
        for i in 1..6 {
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

/// 一个四边形的四个顶点：`p` 四角按 0-1-2-3 绕一圈，`uv`、`e`（到四条边的设备距离）同序
pub fn push_quad(v: &mut Vec<f32>, p: [(f32, f32); 4], uv: [(f32, f32); 4], c: [f32; 4], r: [f32; 4], e: &[[f32; 4]; 4], k: f32) {
    for i in 0..4 {
        let ei = e[i];
        v.extend_from_slice(&[p[i].0, p[i].1, uv[i].0, uv[i].1, c[0], c[1], c[2], c[3], r[0], r[1], r[2], r[3], ei[0], ei[1], ei[2], ei[3], k]);
    }
}

/// 旋转 / 斜切的目标平行四边形（p：左上、右上、右下、左下）做边缘抗锯齿：四边各向外扩 1 个设备像素，
/// uv 按同一个仿射外推，给出每个顶点到四条边的设备距离。退化（面积 0）时原样返回、不抗锯齿
pub fn aa_quad(p: [(f32, f32); 4], uv: [(f32, f32); 4]) -> ([(f32, f32); 4], [(f32, f32); 4], [[f32; 4]; 4]) {
    let Some((hs, ht)) = crate::renderer::components::canvas_image::aa_edges(p[0], p[1], p[3]) else { return (p, uv, NO_EDGE) };
    let (es, et) = (1.0 / hs, 1.0 / ht);
    let at = |s: f32, t: f32| {
        let q = (p[0].0 + s * (p[1].0 - p[0].0) + t * (p[3].0 - p[0].0), p[0].1 + s * (p[1].1 - p[0].1) + t * (p[3].1 - p[0].1));
        let w = (uv[0].0 + s * (uv[1].0 - uv[0].0) + t * (uv[3].0 - uv[0].0), uv[0].1 + s * (uv[1].1 - uv[0].1) + t * (uv[3].1 - uv[0].1));
        (q, w, [s * hs, (1.0 - s) * hs, t * ht, (1.0 - t) * ht])
    };
    let c = [at(-es, -et), at(1.0 + es, -et), at(1.0 + es, 1.0 + et), at(-es, 1.0 + et)];
    (c.map(|x| x.0), c.map(|x| x.1), c.map(|x| x.2))
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
