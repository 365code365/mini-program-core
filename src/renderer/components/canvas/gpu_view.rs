//! 给小游戏 GPU 2D 后端（game/canvas/gpu2d）看的那几样状态，与「换一张后备缓冲来画」。
//!
//! GPU 后端不另写一份光栅器：形状、文字、渐变、阴影仍由这里的 CPU 代码画，只是画进一张**透明的临时层**
//! （`swap_backing` 换进来），画完把被写过的那块当纹理交给 GPU 按合成模式叠上去。所以它要知道的只是
//! 「这一笔会落在哪」（外接框）与「这一笔怎么叠」（合成模式、裁剪、阴影），都在这里给出，不碰别的字段。

use super::fx2d::Composite;
use super::*;

/// 合成模式里 GPU 能用固定管线混合函数表达的那几种（预乘空间的 Porter-Duff）；其余（multiply 这类要读目标色的）走 CPU
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuBlend {
    SourceOver,
    SourceIn,
    SourceOut,
    SourceAtop,
    DestinationOver,
    DestinationIn,
    DestinationOut,
    DestinationAtop,
    Lighter,
    Copy,
    Xor,
}

impl GpuBlend {
    /// 会动到形状**之外**的像素（源覆盖率为 0 处目标也变）：GPU 上要铺满整块画布画
    pub fn clears_outside(self) -> bool {
        matches!(self, Self::SourceIn | Self::SourceOut | Self::DestinationIn | Self::DestinationAtop | Self::Copy)
    }
}

impl Canvas2DContext {
    /// 当前合成模式在 GPU 上的混合；None = 要读目标色（multiply / screen / darken / lighten / difference）
    pub fn gpu_blend(&self) -> Option<GpuBlend> {
        Some(match self.fx.composite {
            Composite::SourceOver => GpuBlend::SourceOver,
            Composite::SourceIn => GpuBlend::SourceIn,
            Composite::SourceOut => GpuBlend::SourceOut,
            Composite::SourceAtop => GpuBlend::SourceAtop,
            Composite::DestinationOver => GpuBlend::DestinationOver,
            Composite::DestinationIn => GpuBlend::DestinationIn,
            Composite::DestinationOut => GpuBlend::DestinationOut,
            Composite::DestinationAtop => GpuBlend::DestinationAtop,
            Composite::Lighter => GpuBlend::Lighter,
            Composite::Copy => GpuBlend::Copy,
            Composite::Xor => GpuBlend::Xor,
            _ => return None,
        })
    }

    pub fn gpu_transform(&self) -> [f32; 6] {
        self.transform
    }

    pub fn gpu_alpha(&self) -> f32 {
        self.global_alpha.clamp(0.0, 1.0)
    }

    /// 用户坐标 → 设备坐标
    pub fn gpu_tp(&self, x: f32, y: f32) -> (f32, f32) {
        self.tp(x, y)
    }

    /// 阴影可见时：(模糊, x 偏移, y 偏移)，设备 px
    pub fn gpu_shadow(&self) -> Option<(f32, f32, f32)> {
        let s = self.fx.shadow;
        s.visible().then(|| (s.blur * self.dpr, s.ox * self.dpr, s.oy * self.dpr))
    }

    /// 裁剪蒙版（设备像素，行 0 在上；Arc 的地址可以当缓存键）
    pub fn gpu_clip(&self) -> Option<&Arc<Vec<u8>>> {
        self.clip.as_ref()
    }

    /// 填充是纯色（没有渐变 / 图案）时的颜色（未乘 globalAlpha）
    pub fn gpu_solid_fill(&self) -> Option<Color> {
        self.fx.fill.is_none().then_some(self.fill_style)
    }

    /// 当前路径（设备坐标）所有点的外接框；`stroke` 时按线宽外扩（斜接按 HTML 默认 miterLimit 10 封顶：
    /// 光栅器自己不封顶，超出的那截被临时层的裁剪框切掉，与浏览器的斜接上限是同一个结果）
    pub fn gpu_path_bounds(&self, stroke: bool) -> Option<(f32, f32, f32, f32)> {
        let b = super::fx_mask::path_hull(&self.build_path())?;
        Some(if stroke { pad(b, self.line_width * self.avg_scale() * 5.0) } else { b })
    }

    /// strokeRect 的外接框（设备坐标，已按线宽外扩）
    pub fn gpu_rect_bounds(&self, x: f32, y: f32, w: f32, h: f32, stroke: bool) -> (f32, f32, f32, f32) {
        let pts = [self.tp(x, y), self.tp(x + w, y), self.tp(x + w, y + h), self.tp(x, y + h)];
        let b = pts.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)));
        if stroke { pad(b, self.line_width * self.avg_scale() * 5.0) } else { b }
    }

    /// 一段文字可能落到的外接框（设备坐标，保守）：不量宽（measureText 在真机上一次就要上百微秒），
    /// 按「每个字至多 1.5 个字号宽」、上下各 2 个字号，四周再加描边线宽；对齐方式让文字落在锚点左或右，两边都留
    pub fn gpu_text_bounds(&self, size: f32, chars: usize, max_width: Option<f32>, stroke: bool) -> impl Fn(f32, f32) -> (f32, f32, f32, f32) + '_ {
        let mut w = chars as f32 * size * 1.5 + size;
        if let Some(m) = max_width.filter(|m| m.is_finite() && *m > 0.0) {
            w = w.min(m + size);
        }
        let lw = if stroke { self.line_width * 2.0 } else { 0.0 };
        move |x: f32, y: f32| {
            let (x0, x1, y0, y1) = (x - w - lw, x + w + lw, y - 2.0 * size - lw, y + 2.0 * size + lw);
            let pts = [self.tp(x0, y0), self.tp(x1, y0), self.tp(x1, y1), self.tp(x0, y1)];
            pts.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)))
        }
    }

    /// 后备缓冲换成 `c`（同尺寸）：GPU 后端拿一张透明临时层换进来画一笔，画完再换回去
    pub fn swap_backing(&mut self, c: &mut Canvas) {
        if let Ok(mut mine) = self.canvas.lock() {
            std::mem::swap(&mut *mine, c);
        }
    }

    /// 临时把合成模式当成 source-over 跑 `f`（画进透明临时层时用：真正的合成交给 GPU）
    pub fn with_source_over<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = std::mem::take(&mut self.fx.composite);
        let r = f(self);
        self.fx.composite = saved;
        r
    }
}

fn pad(b: (f32, f32, f32, f32), p: f32) -> (f32, f32, f32, f32) {
    let p = p.max(0.0) + 2.0;
    (b.0 - p, b.1 - p, b.2 + p, b.3 + p)
}
