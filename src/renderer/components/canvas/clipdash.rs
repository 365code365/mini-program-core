//! `clip()`（路径裁剪）与 `setLineDash` / `lineDashOffset`（虚线描边）、`miterLimit`。
//!
//! `Canvas2DContext` 的一片（旧接口 `CanvasContext` 与 `type="2d"` 共用）。
//!
//! - **裁剪**：`clip()` 把当前路径（按当前矩阵）光栅化成一张同尺寸的覆盖率蒙版，与已有的蒙版相乘（HTML 语义：
//!   裁剪区只会越剪越小，随 save / restore 进出）。有裁剪时每条绘制指令照常画，画完按蒙版把这一次的改动与
//!   画之前的像素混合 —— 所有绘制方式（纯色 / 渐变 / 阴影 / 文字 / 图片 / clearRect）一视同仁，不用逐个改。
//!   代价是有裁剪时每条绘制指令多一次整块缓冲的拷贝；没裁剪时一条路径都不多走。
//! - **虚线**：描边前把路径展平成折线，按虚线模式（设备像素，随当前矩阵的平均缩放）切成一段段子路径再描。
//!   奇数个元素按 HTML 规则复制一遍；含负数或非有限值的整条忽略；全 0 等于实线。
//! - **miterLimit**：记下（`getter` 读得回），光栅器的斜接没有长度上限 —— 写明的取舍。
use super::*;

/// 有裁剪时要按蒙版混合的那些指令（只改状态、只建路径的不算）
pub(super) const DRAW_OPS: &[&str] = &[
    "fill", "stroke", "fillRect", "strokeRect", "clearRect", "fillText", "strokeText", "drawImage",
];

impl Canvas2DContext {
    /// `ctx.clip()`：当前路径与已有裁剪区求交
    pub fn clip(&mut self) {
        let path = self.build_path();
        let Ok(canvas) = self.canvas.lock() else { return };
        let (w, h) = (canvas.width(), canvas.height());
        drop(canvas);
        let mut mask = Canvas::new(w, h);
        mask.clear(Color::TRANSPARENT);
        let paint = Paint::new().with_color(Color::WHITE).with_style(PaintStyle::Fill).with_anti_alias(true);
        mask.draw_path(&path, &paint);
        let cov: Vec<u8> = mask.pixels().iter().map(|p| p.a).collect();
        let merged = match &self.clip {
            Some(old) if old.len() == cov.len() => old.iter().zip(cov).map(|(a, b)| ((*a as u16 * b as u16 + 127) / 255) as u8).collect(),
            _ => cov,
        };
        self.clip = Some(Arc::new(merged));
    }

    pub(super) fn has_clip(&self) -> bool {
        self.clip.is_some()
    }

    /// 有裁剪时：画之前留一份像素
    pub(super) fn clip_snapshot(&self) -> Option<Vec<Color>> {
        self.clip.as_ref()?;
        self.canvas.lock().ok().map(|c| c.pixels().to_vec())
    }

    /// 画完：蒙版外还原、蒙版边缘按覆盖率混合
    pub(super) fn clip_apply(&self, before: Vec<Color>) {
        let Some(mask) = &self.clip else { return };
        let Ok(mut canvas) = self.canvas.lock() else { return };
        let px = canvas.pixels_mut();
        if px.len() != before.len() || px.len() != mask.len() {
            return;
        }
        for ((p, b), m) in px.iter_mut().zip(before.iter()).zip(mask.iter()) {
            match *m {
                255 => {}
                0 => *p = *b,
                m => {
                    let k = m as u16;
                    let mix = |a: u8, o: u8| ((a as u16 * k + o as u16 * (255 - k) + 127) / 255) as u8;
                    *p = Color::new(mix(p.r, b.r), mix(p.g, b.g), mix(p.b, b.b), mix(p.a, b.a));
                }
            }
        }
    }

    /// `setLineDash(segments)`：段长是用户坐标里的长度
    pub fn set_line_dash(&mut self, segments: &[f32]) {
        if segments.iter().any(|s| !s.is_finite() || *s < 0.0) {
            return;
        }
        let mut v = segments.to_vec();
        if v.len() % 2 == 1 {
            v.extend_from_slice(segments);
        }
        self.line_dash = if v.iter().all(|s| *s == 0.0) { Vec::new() } else { v };
    }

    pub fn set_line_dash_offset(&mut self, offset: f32) {
        if offset.is_finite() {
            self.dash_offset = offset;
        }
    }

    pub fn set_miter_limit(&mut self, limit: f32) {
        if limit.is_finite() && limit > 0.0 {
            self.miter_limit = limit;
        }
    }

    /// 描边前：有虚线就把路径切成虚线段（设备坐标）
    pub(super) fn dashed(&self, path: Path) -> Path {
        if self.line_dash.is_empty() {
            return path;
        }
        let sc = self.avg_scale().max(1e-3);
        let pattern: Vec<f32> = self.line_dash.iter().map(|d| d * sc).collect();
        let total: f32 = pattern.iter().sum();
        if total <= 0.0 {
            return path;
        }
        let mut out = Path::new();
        for contour in path.flatten(0.2) {
            // 每条子路径从虚线模式的 offset 处重新开始（HTML 语义）
            let mut idx = 0usize;
            let mut left = pattern[0];
            let mut off = (self.dash_offset * sc).rem_euclid(total);
            while off > 0.0 {
                if off >= left {
                    off -= left;
                    idx = (idx + 1) % pattern.len();
                    left = pattern[idx];
                } else {
                    left -= off;
                    off = 0.0;
                }
            }
            let mut pen_down = false;
            for seg in contour.windows(2) {
                let (a, b) = (seg[0], seg[1]);
                let len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
                if len <= 0.0 {
                    continue;
                }
                let mut t = 0.0f32;
                while t < len {
                    let step = left.min(len - t);
                    let on = idx % 2 == 0;
                    let p0 = (a.x + (b.x - a.x) * t / len, a.y + (b.y - a.y) * t / len);
                    let t1 = t + step;
                    let p1 = (a.x + (b.x - a.x) * t1 / len, a.y + (b.y - a.y) * t1 / len);
                    if on {
                        if !pen_down {
                            out.move_to(p0.0, p0.1);
                            pen_down = true;
                        }
                        out.line_to(p1.0, p1.1);
                    } else {
                        pen_down = false;
                    }
                    left -= step;
                    t = t1;
                    if left <= 1e-4 {
                        idx = (idx + 1) % pattern.len();
                        left = pattern[idx];
                        if idx % 2 == 1 {
                            pen_down = false;
                        }
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(ctx: &Canvas2DContext, x: u32, y: u32) -> u8 {
        let c = ctx.canvas.lock().unwrap();
        c.pixels()[(y * c.width() + x) as usize].a
    }

    #[test]
    fn clip_limits_later_fills_and_restore_lifts_it() {
        let mut ctx = Canvas2DContext::new("t", 40, 40);
        ctx.save();
        ctx.begin_path();
        ctx.rect(0.0, 0.0, 20.0, 40.0);
        ctx.clip();
        ctx.set_fill_style("#ff0000");
        let snap = ctx.clip_snapshot().unwrap();
        ctx.fill_rect(0.0, 0.0, 40.0, 40.0);
        ctx.clip_apply(snap);
        assert_eq!(alpha_at(&ctx, 10, 10), 255, "裁剪区内画上了");
        assert_eq!(alpha_at(&ctx, 30, 10), 0, "裁剪区外没画");
        ctx.restore();
        assert!(!ctx.has_clip(), "restore 之后裁剪没了");
    }

    #[test]
    fn dashes_leave_gaps() {
        let mut ctx = Canvas2DContext::new("t", 40, 10);
        ctx.set_line_width(2.0);
        ctx.set_line_dash(&[5.0, 5.0]);
        ctx.begin_path();
        ctx.move_to(0.0, 5.0);
        ctx.line_to(40.0, 5.0);
        ctx.stroke();
        assert!(alpha_at(&ctx, 2, 5) > 200, "第一段实");
        assert!(alpha_at(&ctx, 7, 5) < 40, "第一段空");
        assert!(alpha_at(&ctx, 12, 5) > 200, "第二段实");
    }

    #[test]
    fn odd_dash_list_is_doubled_and_bad_values_ignored() {
        let mut ctx = Canvas2DContext::new("t", 4, 4);
        ctx.set_line_dash(&[1.0, 2.0, 3.0]);
        assert_eq!(ctx.line_dash, vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0]);
        ctx.set_line_dash(&[1.0, -1.0]);
        assert_eq!(ctx.line_dash.len(), 6, "含负数的整条忽略");
        ctx.set_line_dash(&[]);
        assert!(ctx.line_dash.is_empty());
    }
}

impl Canvas2DContext {
    /// 一次绘制（小游戏的直调路径，game/canvas）：有裁剪区时与 JSON 解释器（commands.rs）同样先拍快照、画完按蒙版混合，
    /// 所以两条路的像素结果相同。只对绘制类调用用（DRAW_OPS 里那些），设状态的调用直接调方法
    pub fn draw_op(&mut self, f: impl FnOnce(&mut Self)) {
        if self.has_clip() {
            let before = self.clip_snapshot();
            f(self);
            if let Some(b) = before {
                self.clip_apply(b);
            }
        } else {
            f(self);
        }
    }

    /// 执行一条与 JSON 解释器同格式的指令（渐变 / 图案 / 阴影 / 合成模式这类不在热路径上的设置，
    /// 小游戏直接复用解释器那份解析，免得两份实现漂移）
    pub fn run_json_command(&mut self, cmd: &serde_json::Value) {
        super::commands::run_command(self, cmd);
    }
}

impl Canvas2DContext {
    /// save() 栈当前的深度（小游戏限 1024 层，game/canvas/natives.rs）
    pub fn save_depth(&self) -> usize {
        self.state_stack.len()
    }

    /// 当前字号（逻辑 px，measureText 用）
    pub fn font_size_px(&self) -> f32 {
        self.font_size
    }
}
