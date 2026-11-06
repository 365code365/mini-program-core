//! 动画底图：一块只因为动画在重画的矩形，动画节点底下那一层留着，下一帧直接贴回、不再画一遍。
//!
//! 为什么要它：局部重画一块矩形时，块里落着的节点按绘制顺序**全部**重画 —— 转圈的唱片（播放页）、
//! 加载圈底下是什么都要跟着画：整页的模糊封面图（`opacity: .55` 成组透明度，离屏 + 逐像素混合）、
//! 一层渐变遮罩……这些一个像素都没变，却每帧占掉这块矩形一半以上的时间。Skyline 靠图层合成
//! 拿到这一点（动画节点单独一层、底下的层不重画）；CPU 后端这里用「留底图」做到同一件事。
//!
//! 做法（一块矩形一趟，见 engine_frame.rs 的 `PagePass::Rects`）：
//! 1. **留**：这块矩形没碰到这一帧动画以外的损伤（[`WxmlRenderer::set_backdrop_others`]），这一趟画到某个动画节点 X
//!    时 —— X 的包围盒在这块矩形里、它之前画过的东西里没有别的动画碰到这块、X 不在成组透明度里、画的是页面画布 ——
//!    把这块矩形此刻的像素（= X 底下的全部）拷一份；
//! 2. **用**：下一帧又是同一块矩形、又没碰到别的损伤：X 之前的节点照常遍历（登记命中区、事件绑定）但不画
//!    （`outside_damage` 一律当成在损伤区外，成组透明度不开离屏），走到 X 时把留的那份贴回，之后照常画。
//!    X 之后（压在上面）的节点照常画；
//! 3. **作废**：整帧重画、一块别的矩形碰到它、换页 / 图片下完 / 字体换了（`invalidate_offscreen_caches`）、
//!    这一帧别的损伤碰到了它 —— 都丢掉。用的那一趟如果没走到 X、或者 X 之前冒出了碰到这块的动画，
//!    这一趟作废，宿主清掉重画一遍（[`WxmlRenderer::take_backdrop_retry`]）。
//!
//! 为什么「没碰到别的损伤」就够：局部重画本来就靠「损伤区外的像素与上一帧相同」成立；底图是同一个道理落在
//! 矩形里面 —— X 底下要是有东西变了，失效分析 / 点击态 / 动图都会报出碰到这块的损伤。拖进度条时回调改时间文字、
//! 滑块那块在别处，唱片那块照样贴底图。
//! 判据：`tools/damage-check.sh`、`tests/sdk_damage_*`（局部重画 = 整帧重画，逐字节）。`MINI_NO_BACKDROP=1` 关掉对照。
use super::*;

thread_local! {
    /// 本线程贴回过几次底图（测试用：确认真的走到了这条路，不是两边都在照常画）
    static RESTORES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// 本线程贴回过几次动画底图（测试的空转守卫）
pub fn backdrop_restores() -> u64 {
    RESTORES.with(|c| c.get())
}

struct Saved {
    rect: (i32, i32, i32, i32),
    node: NodeId,
    pixels: Vec<Color>,
}

#[derive(Default)]
pub(super) struct BackdropState {
    /// 宿主：这一帧动画以外的损伤（None = 这一帧不许用，例如整条带）
    others: Option<Vec<GeoRect>>,
    /// 这一趟的矩形能不能用 / 留底图（没碰到 `others`）
    allowed: bool,
    saved: Option<Saved>,
    /// 这一趟的矩形（整像素，画布坐标）与画布（地址，只认页面画布）
    pass: Option<((i32, i32, i32, i32), usize)>,
    /// 正在跳过 X 之前的绘制：X 是谁
    mute_target: Option<NodeId>,
    captured: bool,
    retry: bool,
    /// 成组透明度的层数（X 在成组透明度里就不留）
    pub(super) opacity_depth: u32,
}

fn int_rect(r: GeoRect, w: u32, h: u32) -> (i32, i32, i32, i32) {
    let x0 = (r.x.floor() as i32).clamp(0, w as i32);
    let y0 = (r.y.floor() as i32).clamp(0, h as i32);
    let x1 = ((r.x + r.width).ceil() as i32).clamp(x0, w as i32);
    let y1 = ((r.y + r.height).ceil() as i32).clamp(y0, h as i32);
    (x0, y0, x1, y1)
}

fn meets(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

fn meets_f(a: (i32, i32, i32, i32), r: &GeoRect) -> bool {
    (r.x as i32) < a.2 && a.0 < (r.x + r.width).ceil() as i32 && (r.y as i32) < a.3 && a.1 < (r.y + r.height).ceil() as i32
}

impl WxmlRenderer {
    /// 宿主每帧画页面之前调：这一帧动画以外的损伤（数据 / 点击态、动图换帧、补条带、吸顶）。
    /// 碰到这些的那块矩形不用底图（底下可能变了）；`None` 整帧都不用
    pub fn set_backdrop_others(&mut self, others: Option<Vec<GeoRect>>) {
        self.backdrop.others = others.filter(|_| std::env::var_os("MINI_NO_BACKDROP").is_none());
    }

    /// 这一趟用底图失败了（没走到 X / X 底下冒出了动画）：宿主把这块清掉重画一遍。底图已经丢掉，重画那趟照常画
    pub fn take_backdrop_retry(&mut self) -> bool {
        std::mem::take(&mut self.backdrop.retry)
    }

    /// 底图整个丢掉（图片下完、字体换了、换页）
    pub(super) fn drop_backdrop(&mut self) {
        self.backdrop.saved = None;
    }

    /// 页面那一趟开始（损伤区已设好）
    pub(super) fn backdrop_begin_pass(&mut self, canvas: &Canvas) {
        let b = &mut self.backdrop;
        b.captured = false;
        b.mute_target = None;
        self.backdrop_mute = false;
        let Some(d) = self.damage_clip else {
            // 整帧：底下什么都可能变了
            b.saved = None;
            b.pass = None;
            return;
        };
        let rect = int_rect(d, canvas.width(), canvas.height());
        b.pass = Some((rect, canvas as *const Canvas as usize));
        b.allowed = b.others.as_ref().is_some_and(|o| !o.iter().any(|r| meets_f(rect, r)));
        match &b.saved {
            Some(s) if b.allowed && s.rect == rect && s.pixels.len() == ((rect.2 - rect.0) * (rect.3 - rect.1)) as usize => {
                b.mute_target = Some(s.node);
                self.backdrop_mute = true;
            }
            Some(s) if meets(s.rect, rect) => b.saved = None,
            _ => {}
        }
    }

    /// 页面那一趟画完
    pub(super) fn backdrop_end_pass(&mut self) {
        if self.backdrop_mute {
            // 没走到 X（它不在了 / 被别的路径画了）：这一趟 X 之前的东西都没画，作废重画
            self.backdrop_mute = false;
            self.backdrop.retry = true;
            self.backdrop.saved = None;
        }
        self.backdrop.mute_target = None;
        self.backdrop.pass = None;
    }

    /// 走到一个自己在动的节点、它的包围盒刚记进 `animated_bounds`（最后一项），还没开始画
    pub(super) fn backdrop_at_animated(&mut self, canvas: &mut Canvas, node: NodeId) {
        let Some((rect, ptr)) = self.backdrop.pass else { return };
        if canvas as *const Canvas as usize != ptr {
            return;
        }
        let Some((own, before)) = self.animated_bounds.split_last() else { return };
        // 这一趟里它之前已经有别的东西在这块里动（底图里那块就不是定的）
        let beneath = before.iter().any(|r| meets_f(rect, r));
        if self.backdrop_mute {
            if self.backdrop.mute_target == Some(node) && !beneath {
                if let Some(s) = &self.backdrop.saved {
                    let w = (rect.2 - rect.0) as usize;
                    canvas.copy_opaque_rows(rect.0, rect.1, &s.pixels, w);
                    RESTORES.with(|c| c.set(c.get() + 1));
                }
                self.backdrop_mute = false;
            } else if beneath || meets_f(rect, own) {
                // X 之前冒出来一个碰到这块的动画：底图不可信，这一趟作废重画
                self.backdrop_mute = false;
                self.backdrop.retry = true;
                self.backdrop.saved = None;
            }
            return;
        }
        let b = &mut self.backdrop;
        let inside = own.x >= rect.0 as f32 && own.y >= rect.1 as f32 && own.x + own.width <= rect.2 as f32 && own.y + own.height <= rect.3 as f32;
        if !b.allowed || b.captured || b.saved.is_some() || beneath || !inside || b.opacity_depth > 0 {
            return;
        }
        b.captured = true;
        let (cw, w) = (canvas.width() as usize, (rect.2 - rect.0) as usize);
        let px = canvas.pixels();
        let mut pixels = Vec::with_capacity(w * (rect.3 - rect.1) as usize);
        for y in rect.1..rect.3 {
            let at = y as usize * cw + rect.0 as usize;
            pixels.extend_from_slice(&px[at..at + w]);
        }
        b.saved = Some(Saved { rect, node, pixels });
    }

    /// 底图占的像素缓冲（内存分项用）
    pub(super) fn backdrop_buffer(&self) -> Option<&[Color]> {
        self.backdrop.saved.as_ref().map(|s| s.pixels.as_slice())
    }
}
