//! 一个 WebGL 上下文（design-webgl.md §2、§3）：EGL 句柄、GL 函数表、属性、WebGL 自己的那部分状态。

use super::egl::{self, Display, Got, Want};
use glow::HasContext;
use khronos_egl as k;

/// WebGL 独有、不下发 GL 的像素存储参数（pixels.rs 用）
#[derive(Clone, Copy, Debug)]
pub struct Unpack {
    pub flip_y: bool,
    pub premultiply: bool,
    /// UNPACK_COLORSPACE_CONVERSION_WEBGL（只记、读回；不做色彩管理）
    pub colorspace: u32,
    pub alignment: i32,
    // WebGL2
    pub row_length: i32,
    pub image_height: i32,
    pub skip_pixels: i32,
    pub skip_rows: i32,
    pub skip_images: i32,
}

impl Default for Unpack {
    fn default() -> Self {
        Self { flip_y: false, premultiply: false, colorspace: 0x9244, alignment: 4, row_length: 0, image_height: 0, skip_pixels: 0, skip_rows: 0, skip_images: 0 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Attrs {
    pub alpha: bool,
    pub depth: bool,
    pub stencil: bool,
    pub antialias: bool,
    pub premultiplied_alpha: bool,
    pub preserve_drawing_buffer: bool,
}

pub struct GlCtx {
    pub d: &'static Display,
    cfg: k::Config,
    context: k::Context,
    surface: k::Surface,
    pub gl: glow::Context,
    /// 1 = WebGL1，2 = WebGL2
    pub version: u8,
    pub attrs: Attrs,
    pub got: Got,
    pub w: u32,
    pub h: u32,
    /// 自己生成的错误（WebGL：每种错误码只记一个，getError 先报这些）
    pub errors: Vec<u32>,
    pub lost: bool,
    /// 丢失后第一次 getError 报 CONTEXT_LOST_WEBGL
    pub lost_reported: bool,
    pub unpack: Unpack,
    pub pack_alignment: i32,
    /// 默认帧缓冲自上次上屏以来被画过 / 清过（帧末要读回）
    pub drew: bool,
    /// preserveDrawingBuffer=false：上屏之后、下一次碰默认帧缓冲之前要清一次
    pub needs_clear: bool,
    /// 当前画 / 读的帧缓冲是不是默认的（bindFramebuffer 时更新）
    pub draw_fb_default: bool,
    pub read_fb_default: bool,
    /// 已经开了的 WebGL 扩展名
    pub enabled: Vec<String>,
    /// 不是 ANGLE 的 WebGL 上下文时，绘制前的内存安全校验（validate.rs）
    pub check: Option<super::validate::Tracker>,
    /// 读回用的缓冲（每帧复用，不分配）
    pub scratch: Vec<u8>,
    /// WebGLSync：GLsync 是指针，不交给 JS，JS 只拿编号
    pub syncs: std::collections::HashMap<u32, glow::NativeFence>,
    pub next_sync: u32,
    /// 按 WebGL1 的 DEPTH_STENCIL 分配的渲染缓冲（GL 里是 DEPTH24_STENCIL8，查询时报回 DEPTH_STENCIL）与当前绑着的那个
    pub ds_rbos: std::collections::HashSet<u32>,
    pub bound_rbo: u32,
    /// 当前当默认帧缓冲用的窗口 surface（window.rs 建、毁；None = pbuffer）
    window: Option<k::Surface>,
}

impl GlCtx {
    pub fn new(d: &'static Display, version: u8, attrs: Attrs, w: u32, h: u32) -> Result<Self, String> {
        let es3 = version == 2;
        let want = Want { alpha: attrs.alpha, depth: attrs.depth, stencil: attrs.stencil, antialias: attrs.antialias, es3 };
        let (cfg, got) = egl::choose_config(d, &want).ok_or("没有合用的 EGL config")?;
        let context = egl::create_context(d, cfg, es3, true)?;
        let surface = match egl::create_pbuffer(d, cfg, w, h) {
            Ok(s) => s,
            Err(e) => {
                let _ = d.egl.destroy_context(d.dpy, context);
                return Err(e);
            }
        };
        d.egl.make_current(d.dpy, Some(surface), Some(surface), Some(context)).map_err(|e| format!("eglMakeCurrent: {e}"))?;
        let gl = unsafe { glow::Context::from_loader_function(|s| egl::proc_address(d, s)) };
        let check = (!d.webgl_compat).then(super::validate::Tracker::default);
        let mut c = Self { d, cfg, context, surface, gl, version, attrs, got, w, h, errors: Vec::new(), lost: false, lost_reported: false,
            unpack: Unpack::default(), pack_alignment: 4, drew: false, needs_clear: false, draw_fb_default: true, read_fb_default: true,
            enabled: Vec::new(), check, scratch: Vec::new(), syncs: Default::default(), next_sync: 0, window: None, ds_rbos: Default::default(), bound_rbo: 0 };
        unsafe {
            // WebGL：视口与裁剪框的初值 = 画布尺寸（GL 也是 surface 尺寸，pbuffer 就是画布尺寸）
            c.gl.viewport(0, 0, w as i32, h as i32);
            c.gl.scissor(0, 0, w as i32, h as i32);
            if !d.webgl_compat {
                // 系统 GLES：WebGL 要求默认帧缓冲一开始是清过的（ANGLE 的资源零初始化管这个）
                c.gl.clear_color(0.0, 0.0, 0.0, 0.0);
                c.gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT | glow::STENCIL_BUFFER_BIT);
            }
        }
        c.attrs.alpha = got.alpha;
        c.attrs.depth = got.depth;
        c.attrs.stencil = got.stencil;
        c.attrs.antialias = got.samples > 0;
        Ok(c)
    }

    pub fn config(&self) -> k::Config {
        self.cfg
    }

    /// 默认帧缓冲换成 `win`（Android 直接上屏的窗口 surface，window.rs）；None = 回到 pbuffer
    pub fn bind_window(&mut self, win: Option<k::Surface>) {
        self.window = win;
        self.make_current();
    }

    pub fn make_current(&mut self) -> bool {
        if self.lost {
            return false;
        }
        let s = self.window.unwrap_or(self.surface);
        if self.d.egl.make_current(self.d.dpy, Some(s), Some(s), Some(self.context)).is_err() {
            // EGL_CONTEXT_LOST / BAD_*：都按丢失处理，游戏收 webglcontextlost
            self.lose();
            return false;
        }
        true
    }

    pub fn lose(&mut self) {
        self.lost = true;
        self.lost_reported = false;
    }

    /// 画布改尺寸：换 pbuffer（上下文与资源保留，视口不动 —— 与浏览器一致）
    pub fn resize(&mut self, w: u32, h: u32) -> Result<(), String> {
        if (w, h) == (self.w, self.h) || self.lost {
            return Ok(());
        }
        let s = egl::create_pbuffer(self.d, self.cfg, w, h)?;
        let _ = self.d.egl.make_current(self.d.dpy, Some(s), Some(s), Some(self.context));
        let _ = self.d.egl.destroy_surface(self.d.dpy, self.surface);
        self.surface = s;
        (self.w, self.h) = (w, h);
        // 窗口 surface 当默认帧缓冲时（window.rs）：尺寸跟着宿主的 setBuffersGeometry 走，这里只换回它
        self.make_current();
        // 新 surface 的内容未定义：按 WebGL 清成 0
        self.clear_default_now();
        Ok(())
    }

    /// WebGL 错误：同一个码只记一个
    pub fn error(&mut self, code: u32) {
        if !self.errors.contains(&code) {
            self.errors.push(code);
        }
    }

    /// getError：丢失 → 先报一次 CONTEXT_LOST_WEBGL；再报自己记的；最后问 GL
    pub fn get_error(&mut self) -> u32 {
        if self.lost {
            if !self.lost_reported {
                self.lost_reported = true;
                return 0x9242;
            }
            return 0;
        }
        if !self.errors.is_empty() {
            return self.errors.remove(0);
        }
        unsafe { self.gl.get_error() }
    }

    /// 要画 / 清 / 读默认帧缓冲之前：上一帧上屏后还没清的话先清（preserveDrawingBuffer=false）
    pub fn touch_default(&mut self, draw: bool) {
        let on_default = if draw { self.draw_fb_default } else { self.read_fb_default };
        if !on_default {
            return;
        }
        if self.needs_clear {
            self.clear_default_now();
        }
        if draw {
            self.drew = true;
        }
    }

    /// 按 WebGL 规则清默认帧缓冲：临时复位裁剪、写掩码、清屏值，清完原样还原
    pub fn clear_default_now(&mut self) {
        self.needs_clear = false;
        let gl = &self.gl;
        unsafe {
            let scissor = gl.is_enabled(glow::SCISSOR_TEST);
            let mut cc = [0f32; 4];
            gl.get_parameter_f32_slice(glow::COLOR_CLEAR_VALUE, &mut cc);
            let mut cm = [0i32; 4];
            gl.get_parameter_i32_slice(glow::COLOR_WRITEMASK, &mut cm);
            let depth = gl.get_parameter_f32(glow::DEPTH_CLEAR_VALUE);
            let dmask = gl.get_parameter_i32(glow::DEPTH_WRITEMASK);
            let stencil = gl.get_parameter_i32(glow::STENCIL_CLEAR_VALUE);
            let smask = gl.get_parameter_i32(glow::STENCIL_WRITEMASK);
            let fb = gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING);
            if fb != 0 {
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            }
            gl.disable(glow::SCISSOR_TEST);
            gl.color_mask(true, true, true, true);
            gl.depth_mask(true);
            gl.stencil_mask(0xFFFF_FFFF);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear_depth_f32(1.0);
            gl.clear_stencil(0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT | glow::STENCIL_BUFFER_BIT);
            if scissor {
                gl.enable(glow::SCISSOR_TEST);
            }
            gl.color_mask(cm[0] != 0, cm[1] != 0, cm[2] != 0, cm[3] != 0);
            gl.depth_mask(dmask != 0);
            gl.stencil_mask(smask as u32);
            gl.clear_color(cc[0], cc[1], cc[2], cc[3]);
            gl.clear_depth_f32(depth);
            gl.clear_stencil(stencil);
            if fb != 0 {
                gl.bind_framebuffer(glow::FRAMEBUFFER, std::num::NonZeroU32::new(fb as u32).map(glow::NativeFramebuffer));
            }
        }
    }

    /// 支持的 WebGL 扩展（getSupportedExtensions）
    pub fn supported_extensions(&self) -> Vec<&'static str> {
        let have = self.gl_extensions();
        super::extensions::TABLE
            .iter()
            .filter(|e| (e.version == 0 || e.version == self.version) && super::extensions::available(e, &have, self.version, self.d.webgl_compat))
            .map(|e| e.name)
            .collect()
    }

    /// GL 层的扩展：ANGLE 的 WebGL 上下文里是「可请求的」那张表，其余是 GL_EXTENSIONS
    pub fn gl_extensions(&self) -> Vec<String> {
        unsafe {
            let mut v: Vec<String> = self.gl.supported_extensions().iter().cloned().collect();
            if self.d.webgl_compat {
                let req = self.gl.get_parameter_string(0x93A8);
                v.extend(req.split(' ').filter(|s| !s.is_empty()).map(str::to_string));
            }
            v
        }
    }

    /// getExtension(name)：认得且可用就开（ANGLE 上 glRequestExtensionANGLE），返回是否成功
    pub fn enable_extension(&mut self, name: &str) -> bool {
        let lname = name.to_ascii_lowercase();
        if let Some(e) = self.enabled.iter().find(|e| e.to_ascii_lowercase() == lname) {
            let _ = e;
            return true;
        }
        let Some(e) = super::extensions::TABLE.iter().find(|e| e.name.to_ascii_lowercase() == lname && (e.version == 0 || e.version == self.version)) else {
            return false;
        };
        let have = self.gl_extensions();
        if !super::extensions::available(e, &have, self.version, self.d.webgl_compat) {
            return false;
        }
        if self.d.webgl_compat {
            let f = egl::proc_address(self.d, "glRequestExtensionANGLE");
            if !f.is_null() {
                // SAFETY: void glRequestExtensionANGLE(const GLchar*)
                let f: unsafe extern "C" fn(*const std::ffi::c_char) = unsafe { std::mem::transmute(f) };
                let enabled_now = self.gl.supported_extensions().clone();
                for g in e.gl {
                    if !enabled_now.contains(*g) && have.iter().any(|h| h == g) {
                        let cs = std::ffi::CString::new(*g).unwrap();
                        unsafe { f(cs.as_ptr()) };
                    }
                }
                // 请求过后 GL_EXTENSIONS 变了：glow 缓存了那张表，重取一次函数表与扩展
                self.gl = unsafe { glow::Context::from_loader_function(|s| egl::proc_address(self.d, s)) };
            }
        }
        self.enabled.push(e.name.to_string());
        true
    }

    pub fn has_enabled(&self, name: &str) -> bool {
        self.enabled.iter().any(|e| e == name)
    }
}

impl Drop for GlCtx {
    fn drop(&mut self) {
        let d = self.d;
        // 同步对象、缓冲、纹理……随上下文一起释放（eglDestroyContext），不必逐个删
        let _ = d.egl.make_current(d.dpy, None, None, None);
        let _ = d.egl.destroy_surface(d.dpy, self.surface);
        let _ = d.egl.destroy_context(d.dpy, self.context);
    }
}
