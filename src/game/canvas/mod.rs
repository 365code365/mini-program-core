//! 小游戏的画布（game-doc/02 §5.1）：按实例持有，**不进**进程级 `CANVAS_MANAGER`。
//!
//! - 第一次 `wx.createCanvas()` 是上屏画布（默认尺寸 = 窗口逻辑尺寸），之后的都是离屏（默认 300×150，HTML 默认）；
//! - 改 `width/height` = 清空后备缓冲、状态复位（HTML 语义）；单边上限 4096；
//! - 绘制是**立即模式**：原生调用返回时像素已经写进后备缓冲，没有 JSON 指令、没有指令日志
//!   （小程序那条路要记日志是为了布局重建后重放；游戏画布不会因布局重建，不需要）。

pub mod compose;
/// 2D 画布走 GPU（MINI_2D，见文件头）
#[cfg(feature = "webgl")]
pub mod gpu2d;
mod image_ops;
mod natives;
mod text_ops;

use crate::renderer::components::Canvas2DContext;

pub use natives::register;

/// 单边上限（与小程序的离屏画布一致）
pub const MAX_SIDE: u32 = 4096;
/// HTML 离屏画布的默认尺寸
pub const OFFSCREEN_DEFAULT: (u32, u32) = (300, 150);

pub struct Slot {
    pub ctx: Canvas2DContext,
    /// `canvas.width / height`（可以是 0；后备缓冲至少 1×1）
    pub w: u32,
    pub h: u32,
    /// 拿过的上下文种类：0 = 还没拿，1 = 2d，2 = webgl，3 = webgl2（game/webgl）
    pub kind: u8,
    /// 当前 `font`（解析好的；save / restore 由 JS 外壳在 restore 之后重设）
    pub font: crate::renderer::components::FontSpec,
    /// 内容代数：像素每变一次加一（GPU 2D 把 CPU 画布当纹理源时据此判断要不要重传，gpu2d）
    pub gen: u64,
    /// 走 GPU 的 2D 画布（gpu2d）：哪一边的像素是新的；None = CPU 画布
    pub gpu: Option<Auth>,
}

/// GPU 2D 画布（gpu2d）的像素哪边是新的
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Auth {
    /// 两边一样
    Both,
    /// GPU 画过、CPU 后备缓冲是旧的
    Gpu,
    /// CPU 画过、纹理是旧的
    Cpu,
}

/// 2D 画布用哪个后端（`MINI_2D` / 创建选项 `canvas2d`）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode2d {
    /// 全部 CPU（参考实现）
    Cpu,
    /// 全部 2D 画布走 GPU（建不了上下文就整个回退 CPU）
    Gpu,
    /// 移动端（iOS / Android）的主画布走 GPU、离屏画布 CPU；桌面与无头全 CPU
    #[default]
    Auto,
}

impl Mode2d {
    pub fn parse(s: Option<&str>) -> Self {
        let env = std::env::var("MINI_2D").ok();
        match env.as_deref().or(s).map(str::trim) {
            Some("cpu") | Some("off") => Self::Cpu,
            Some("gpu") => Self::Gpu,
            _ => Self::Auto,
        }
    }
}

impl Slot {
    fn new(id: u32, w: u32, h: u32) -> Self {
        Self { ctx: Canvas2DContext::new_with_dpr(&format!("game#{id}"), w.max(1), h.max(1), 1.0), w, h, kind: 0, font: Default::default(), gen: 0, gpu: None }
    }
}

#[derive(Default)]
pub struct GameCanvases {
    slots: Vec<Option<Slot>>,
    main: Option<u32>,
    /// 主画布自上次合成以来被画过 / 改过尺寸
    pub main_dirty: bool,
    /// 窗口逻辑尺寸（主画布的默认尺寸）
    pub window: (u32, u32),
    /// 主域与开放数据域共享的 sharedCanvas（G9.7，game/opendata.rs）
    pub shared: Option<u32>,
    /// 2D 后端（gpu2d）
    pub mode2d: Mode2d,
    /// 引擎内旋转 / 无头：auto 模式下主画布不走 GPU（输出帧要在 CPU 上转 / 逐字节基线）
    pub cpu_output: bool,
    /// WebGL 后端偏好（GPU 2D 用同一套顺序选 display）
    pub gl_pref: String,
    #[cfg(feature = "webgl")]
    pub gpu2d: Option<Box<gpu2d::Gpu2d>>,
    /// GPU 2D 建不了 / 丢了：这个实例里不再试
    pub gpu2d_failed: bool,
    /// 要进游戏日志的说明（后端、回退原因）
    pub notes: Vec<String>,
}

impl GameCanvases {
    pub fn new(window: (u32, u32)) -> Self {
        Self { window, ..Default::default() }
    }

    /// `wx.createCanvas()`：返回 (id, 宽, 高, 是否上屏)
    pub fn create(&mut self) -> (u32, u32, u32, bool) {
        let id = self.slots.len() as u32;
        let main = self.main.is_none();
        let (w, h) = if main { self.window } else { OFFSCREEN_DEFAULT };
        self.slots.push(Some(Slot::new(id, w, h)));
        if main {
            self.main = Some(id);
            self.main_dirty = true;
        }
        (id, w, h, main)
    }

    /// sharedCanvas：第一次要时建（离屏，默认窗口逻辑尺寸），之后同一块
    pub fn shared(&mut self) -> (u32, u32, u32) {
        if let Some(id) = self.shared {
            if let Some(s) = self.get(id) {
                return (id, s.w, s.h);
            }
        }
        let id = self.slots.len() as u32;
        let (w, h) = self.window;
        self.slots.push(Some(Slot::new(id, w, h)));
        self.shared = Some(id);
        (id, w, h)
    }

    /// 改尺寸：夹到上限，重建后备缓冲（清空 + 状态复位）。返回实际生效的尺寸
    pub fn resize(&mut self, id: u32, w: f64, h: f64) -> Option<(u32, u32)> {
        let clamp = |v: f64| if v.is_finite() && v > 0.0 { (v as u64).min(MAX_SIDE as u64) as u32 } else { 0 };
        let (w, h) = (clamp(w), clamp(h));
        let slot = self.slots.get_mut(id as usize)?.as_mut()?;
        let (kind, gen, gpu) = (slot.kind, slot.gen, slot.gpu);
        *slot = Slot::new(id, w, h);
        slot.kind = kind;
        slot.gen = gen + 1;
        // GPU 画布：纹理按新尺寸重建（清空），两边都是空的
        slot.gpu = gpu.map(|_| Auth::Both);
        #[cfg(feature = "webgl")]
        if let Some(g) = self.gpu2d.as_mut() {
            g.free(id);
        }
        if self.main == Some(id) {
            self.main_dirty = true;
        }
        Some((w, h))
    }

    pub fn get(&mut self, id: u32) -> Option<&mut Slot> {
        self.slots.get_mut(id as usize)?.as_mut()
    }

    pub fn main_id(&self) -> Option<u32> {
        self.main
    }

    pub fn is_main(&self, id: u32) -> bool {
        self.main == Some(id)
    }

    /// JS 对象被回收：释放后备缓冲（一个 4096² 的离屏画布 64MB，泄漏几个就 OOM）。主画布不释放
    pub fn free(&mut self, id: u32) {
        if self.main != Some(id) {
            if let Some(s) = self.slots.get_mut(id as usize) {
                *s = None;
            }
            #[cfg(feature = "webgl")]
            if let Some(g) = self.gpu2d.as_mut() {
                g.free(id);
            }
        }
    }

    pub fn main_slot(&self) -> Option<&Slot> {
        self.slots.get(self.main? as usize)?.as_ref()
    }

    /// 还活着的画布个数（泄漏检查用）
    pub fn live(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }
}
