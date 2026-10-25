//! 宿主承载的原生组件（创建选项 `nativeComponents`：video / map …）**被引擎自己画的层
//! 挡住了多少**，随 `nativeViews` 事件交给宿主。
//!
//! ## 为什么要引擎来算
//! 原生组件是宿主的平台控件，摆在引擎画面（一张纹理 / 一块缓冲）的**上面**。而引擎自己
//! 还画着好几层本该盖住它的东西：picker 面板与遮罩、`wx.showModal` 的遮罩与弹窗、
//! Toast / Loading、`position:fixed` 覆盖层、tabBar。宿主不知道这些层在哪，只能把控件
//! 摆在最上面 —— 于是 picker 面板弹起来，视频压在面板和遮罩上面。
//!
//! 微信（Skyline / 同层渲染）里原生组件插在渲染树里，按层叠顺序被这些层正常盖住。
//! 这里做不到同层：引擎输出是**不透明**的一整块，没法在视频那块挖一个透明的洞让底下的
//! 平台控件透上来。所以退一步：引擎把遮挡的**几何**告诉宿主，宿主照着裁。
//!
//! | 字段 | 含义 | 宿主怎么做 |
//! |---|---|---|
//! | `clip` | 控件还露在外面的矩形（视口逻辑坐标） | 只在这块里画、只在这块里收触摸。宽或高为 0 = 整个被挡住：**控件留着别销毁**（播放进度、地图状态都在里面） |
//! | `holes` | `clip` 里面还被盖住的块（弹窗盒子、Toast、fixed 元素） | 挖掉，露出底下的引擎画面；触摸交给引擎 |
//! | `mask` | 盖在控件上的半透明黑遮罩的不透明度（picker 面板、showModal） | 叠一层这个不透明度的黑色；`> 0` 时控件不收触摸 —— 遮罩吃掉触摸，点遮罩是关面板 |
//!
//! 被裁掉的那部分，引擎画面里本来就是控件底下的占位（video 是一块黑底）再叠上那几层，
//! 所以宿主一裁，看到的就是「面板盖住了视频」。
//!
//! ## 能「削边」的削边，削不了的挖洞
//! 底部面板、吸底操作栏、顶部吸顶栏都是横跨控件整个宽度、压住它一条边的盖板：直接把 `clip`
//! 缩掉那一条，宿主一个矩形裁剪就够了。盖在中间的（居中的弹窗盒子、Toast）缩不成矩形，
//! 才放进 `holes`。
//!
//! ## 取舍（与微信不一致的地方）
//! - `position:fixed` 覆盖层按**子树的包围盒**算不透明（与「点击不穿透」用的是同一份
//!   `fixed_hit_regions`）。WXML 自己写的半透明全屏遮罩因此会把正常流里的视频整个挡掉，
//!   看到的是压暗的黑底而不是压暗的视频；逐像素判透明度太贵，不做。
//! - fixed 层里的原生组件只算引擎覆盖层（picker / 弹窗 / Toast / Loading），不算别的
//!   fixed 子树 —— fixed 子树之间按 z-index 排，这里没有那份顺序。
//! - scroll-view 的裁剪不在这里：原生组件登记时没带祖先的裁剪框（见 `host_regions.rs`）。
use super::engine::MiniEngine;
use crate::Rect;

/// 浮点容差（逻辑像素）：面板顶边、fixed 区域都是算出来的，差一丝不该算「没盖满」
const EPS: f32 = 0.01;

/// 一个原生组件这一帧的可见性（视口逻辑坐标）
#[derive(Debug, Clone, PartialEq)]
pub struct Visibility {
    /// 还露在外面的矩形；宽或高为 0 表示整个被挡住
    pub clip: Rect,
    /// `clip` 里还被盖住、宿主要挖掉的块
    pub holes: Vec<Rect>,
    /// 盖在上面的半透明黑遮罩的不透明度（0 = 没有）
    pub mask: f32,
}

/// 这一帧盖在原生组件上面的东西
#[derive(Debug, Clone, Default)]
pub struct Covers {
    /// 引擎覆盖层（picker 面板、弹窗盒子、Toast、Loading）：压在所有原生组件上
    pub over_all: Vec<Rect>,
    /// `position:fixed` 覆盖层的各棵子树：只压正常流里的原生组件
    pub over_flow: Vec<Rect>,
    /// 全屏半透明黑遮罩合起来的不透明度
    pub mask: f32,
}

impl Covers {
    pub fn visibility(&self, view: Rect, area: Rect, is_fixed: bool) -> Visibility {
        let mut covers = self.over_all.clone();
        if !is_fixed {
            covers.extend_from_slice(&self.over_flow);
        }
        visibility(view, area, &covers, self.mask)
    }
}

impl Visibility {
    /// 侧滑返回：`edge_back::compose` 在**所有层合成完之后**把整帧右移 `dx`（页面、tabBar、
    /// fixed、Toast / 弹窗 / picker 一起走），所以遮挡的相对关系不变，只是整体平移；
    /// 推出屏幕右沿（`screen_w`）的部分裁掉。推到底时 `clip` 为空但照报 ——
    /// 手势可能取消、滑回来，控件不能被收掉重建（视频进度会丢）。
    pub fn shifted(self, dx: f32, screen_w: f32) -> Self {
        if dx == 0.0 {
            return self;
        }
        let moved = Rect::new(self.clip.x + dx, self.clip.y, self.clip.width, self.clip.height);
        let screen = Rect::new(0.0, moved.y, screen_w, moved.height);
        let Some(clip) = intersect(&moved, &screen) else {
            return Visibility { clip: Rect::new(moved.x, moved.y, 0.0, 0.0), holes: Vec::new(), mask: self.mask };
        };
        let holes = self
            .holes
            .iter()
            .filter_map(|h| intersect(&Rect::new(h.x + dx, h.y, h.width, h.height), &clip))
            .collect();
        Visibility { clip, holes, mask: self.mask }
    }
}

fn intersect(a: &Rect, b: &Rect) -> Option<Rect> {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = a.right().min(b.right());
    let y1 = a.bottom().min(b.bottom());
    (x1 - x0 > EPS && y1 - y0 > EPS).then(|| Rect::new(x0, y0, x1 - x0, y1 - y0))
}

/// `view` 是控件的完整矩形，`area` 是它所在那一层的可见范围（正常流 = 视口去掉 tabBar，
/// fixed = 整个视口），`covers` 是压在它上面的不透明块，`mask` 是压在它上面的遮罩。
pub fn visibility(view: Rect, area: Rect, covers: &[Rect], mask: f32) -> Visibility {
    let hidden = |at: &Rect| Visibility { clip: Rect::new(at.x, at.y, 0.0, 0.0), holes: Vec::new(), mask };
    let Some(mut clip) = intersect(&view, &area) else {
        return hidden(&view);
    };
    // 先削边：横跨整个宽度、压住上沿或下沿的盖板（底部面板、吸底栏），竖向同理。
    // 削完一条，原来不横跨的盖板可能变成横跨了，所以反复做到不再变（每轮面积严格变小）。
    for _ in 0..=covers.len() {
        let mut changed = false;
        for c in covers {
            let Some(i) = intersect(&clip, c) else { continue };
            let spans_w = i.x <= clip.x + EPS && i.right() >= clip.right() - EPS;
            let spans_h = i.y <= clip.y + EPS && i.bottom() >= clip.bottom() - EPS;
            let next = if spans_w && i.y <= clip.y + EPS {
                Rect::new(clip.x, i.bottom(), clip.width, clip.bottom() - i.bottom())
            } else if spans_w && i.bottom() >= clip.bottom() - EPS {
                Rect::new(clip.x, clip.y, clip.width, i.y - clip.y)
            } else if spans_h && i.x <= clip.x + EPS {
                Rect::new(i.right(), clip.y, clip.right() - i.right(), clip.height)
            } else if spans_h && i.right() >= clip.right() - EPS {
                Rect::new(clip.x, clip.y, i.x - clip.x, clip.height)
            } else {
                continue;
            };
            if next.width <= EPS || next.height <= EPS {
                return hidden(&clip);
            }
            clip = next;
            changed = true;
        }
        if !changed {
            break;
        }
    }
    // 剩下的就是压在中间的：挖洞
    let holes = covers.iter().filter_map(|c| intersect(&clip, c)).collect();
    Visibility { clip, holes, mask }
}

impl MiniEngine {
    /// 这一帧盖在原生组件上面的东西（与 `compositor::cpu::compose` 的层叠顺序一致：
    /// 页面 < tabBar < fixed 覆盖层 < Toast / Loading / Modal < picker 面板）
    pub(crate) fn native_covers(&self) -> Covers {
        let sf = self.dpr;
        let (pw, ph) = self.pixel_size();
        let to_logical = |r: Rect| Rect::new(r.x / sf, r.y / sf, r.width / sf, r.height / sf);
        let mut covers = Covers::default();
        let mut keep = 1.0f32;
        for (r, m) in crate::host::ui_overlay::overlay_cover(
            pw,
            ph,
            sf,
            &self.toast,
            &self.loading,
            &self.modal,
            self.text.as_deref(),
        ) {
            covers.over_all.push(to_logical(r));
            keep *= 1.0 - m;
        }
        // 面板退场动画期间（closing）照样画着，照样要挡
        if let Some(s) = self.picker_sheet.as_ref() {
            use crate::host::picker_sheet::{mask_alpha, sheet_top, MASK_VISIBLE_MIN, SHEET_H};
            covers.over_all.push(Rect::new(0.0, sheet_top(s), self.width as f32, SHEET_H));
            let a = mask_alpha(s);
            if a > MASK_VISIBLE_MIN {
                keep *= 1.0 - a;
            }
        }
        // fixed 子树的区域是页面视口坐标：引擎画了导航栏时整体在它下面（画面坐标要加回去）
        let top = self.nav_height();
        let kb_shift = self.keyboard_shift_px() as f32 / self.dpr;
        if let Some(r) = self.renderer.as_ref() {
            covers.over_flow.extend(r.fixed_hit_regions().iter().map(|f| Rect::new(f.x, f.y + top - kb_shift, f.width, f.height)));
        }
        // 引擎画的导航栏压在所有原生组件之上（与微信一致：视频滚到导航栏底下就被它挡住）
        if top > 0.0 {
            covers.over_all.push(Rect::new(0.0, 0.0, self.width as f32, top));
        }
        // 右上角胶囊画在 fixed 层最上面：正常流与 fixed 层里的原生组件都被它压着
        if let Some(r) = self.menu_button_rect() {
            covers.over_all.push(r);
        }
        covers.mask = 1.0 - keep;
        covers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Rect = Rect::new(30.0, 400.0, 300.0, 200.0);
    const AREA: Rect = Rect::new(0.0, 0.0, 375.0, 617.0);

    #[test]
    fn 没东西盖着就是整个控件() {
        let v = visibility(VIEW, AREA, &[], 0.0);
        assert_eq!(v.clip, VIEW);
        assert!(v.holes.is_empty());
    }

    #[test]
    fn 底部面板削掉下沿() {
        // picker 面板：顶边 402，横跨整个视口
        let sheet = Rect::new(0.0, 402.0, 375.0, 265.0);
        let v = visibility(VIEW, AREA, &[sheet], 0.5);
        assert_eq!(v.clip, Rect::new(30.0, 400.0, 300.0, 2.0));
        assert!(v.holes.is_empty(), "横跨整个宽度的盖板不该变成洞：{:?}", v.holes);
        assert_eq!(v.mask, 0.5);
    }

    #[test]
    fn 整个被盖住时_clip_为零但仍在原处() {
        let sheet = Rect::new(0.0, 350.0, 375.0, 400.0);
        let v = visibility(VIEW, AREA, &[sheet], 0.5);
        assert_eq!((v.clip.width, v.clip.height), (0.0, 0.0));
        assert_eq!((v.clip.x, v.clip.y), (30.0, 400.0));
    }

    #[test]
    fn 居中的弹窗盒子挖洞() {
        let modal = Rect::new(47.5, 450.0, 280.0, 100.0);
        let v = visibility(VIEW, AREA, &[modal], 0.5);
        assert_eq!(v.clip, VIEW);
        assert_eq!(v.holes, vec![Rect::new(47.5, 450.0, 280.0, 100.0)]);
    }

    #[test]
    fn 层的可见范围先裁_tabbar_底下不露() {
        // 控件下半截伸进 tabBar（area 底边 617）
        let view = Rect::new(30.0, 517.0, 300.0, 200.0);
        let v = visibility(view, AREA, &[], 0.0);
        assert_eq!(v.clip, Rect::new(30.0, 517.0, 300.0, 100.0));
    }

    #[test]
    fn 削边之后才横跨的盖板也削掉() {
        // 左边一条竖栏只盖住下半截，先被底部栏削短后才变成「横跨整个高度」
        let bottom = Rect::new(0.0, 500.0, 375.0, 200.0);
        let left = Rect::new(0.0, 400.0, 80.0, 100.0);
        let v = visibility(VIEW, AREA, &[left, bottom], 0.0);
        assert_eq!(v.clip, Rect::new(80.0, 400.0, 250.0, 100.0));
        assert!(v.holes.is_empty(), "{:?}", v.holes);
    }

    #[test]
    fn fixed_子树只压正常流的控件() {
        let covers = Covers {
            over_all: vec![],
            over_flow: vec![Rect::new(0.0, 0.0, 375.0, 667.0)],
            mask: 0.0,
        };
        let flow = covers.visibility(VIEW, AREA, false);
        assert_eq!(flow.clip.width * flow.clip.height, 0.0, "正常流里的视频被全屏 fixed 盖住");
        let fixed = covers.visibility(VIEW, AREA, true);
        assert_eq!(fixed.clip, VIEW, "fixed 层自己的视频不被 fixed 子树（含它自己）挡");
    }
}
