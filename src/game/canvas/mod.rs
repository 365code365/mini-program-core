//! 小游戏的画布（game-doc/02 §5.1）：按实例持有，**不进**进程级 `CANVAS_MANAGER`。
//!
//! - 第一次 `wx.createCanvas()` 是上屏画布（默认尺寸 = 窗口逻辑尺寸），之后的都是离屏（默认 300×150，HTML 默认）；
//! - 改 `width/height` = 清空后备缓冲、状态复位（HTML 语义）；单边上限 4096；
//! - 绘制是**立即模式**：原生调用返回时像素已经写进后备缓冲，没有 JSON 指令、没有指令日志
//!   （小程序那条路要记日志是为了布局重建后重放；游戏画布不会因布局重建，不需要）。

pub mod compose;
mod natives;

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
    /// 拿过的上下文种类：0 = 还没拿，1 = 2d（webgl 在 G8）
    pub kind: u8,
}

impl Slot {
    fn new(id: u32, w: u32, h: u32) -> Self {
        Self { ctx: Canvas2DContext::new_with_dpr(&format!("game#{id}"), w.max(1), h.max(1), 1.0), w, h, kind: 0 }
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

    /// 改尺寸：夹到上限，重建后备缓冲（清空 + 状态复位）。返回实际生效的尺寸
    pub fn resize(&mut self, id: u32, w: f64, h: f64) -> Option<(u32, u32)> {
        let clamp = |v: f64| if v.is_finite() && v > 0.0 { (v as u64).min(MAX_SIDE as u64) as u32 } else { 0 };
        let (w, h) = (clamp(w), clamp(h));
        let slot = self.slots.get_mut(id as usize)?.as_mut()?;
        let kind = slot.kind;
        *slot = Slot::new(id, w, h);
        slot.kind = kind;
        if self.main == Some(id) {
            self.main_dirty = true;
        }
        Some((w, h))
    }

    pub fn get(&mut self, id: u32) -> Option<&mut Slot> {
        self.slots.get_mut(id as usize)?.as_mut()
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
