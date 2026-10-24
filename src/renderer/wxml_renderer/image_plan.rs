//! 页面上每张 `<image>` 画的时候会用到的几何（给图片预取用，`components::image::prefetch`）：
//! 地址、mode、框（设备像素，页面画布坐标）、圆角。
//!
//! 预取要提前做出与绘制**同一个缓存键**的降采样图与缩放结果（`Canvas::image_mip` / `image_scaled`），
//! 键里有框的尺寸、圆角、四分之一像素的亚像素偏移 —— 这几样只有布局树里才有准数。
//! 位置按正常流累加（与 `draw` 一样：父原点 + taffy 的 location）。scroll-view / transform /
//! 动画 / fixed 的子树画的位置不等于这里算的，跳过（它们第一次出现时照旧同步做）。
use super::*;

/// 一张图的绘制几何（设备像素）
#[derive(Debug, Clone, PartialEq)]
pub struct ImagePlan {
    pub src: String,
    pub mode: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
}

impl WxmlRenderer {
    /// 当前布局树里的图（文档顺序）。还没布局过返回空
    pub fn image_plans(&self) -> Vec<ImagePlan> {
        let Some(cache) = self.cache.as_ref() else { return Vec::new() };
        let mut out = Vec::new();
        for n in &cache.render_nodes {
            Self::walk_images(n, &cache.taffy, (0.0, 0.0), &mut out);
        }
        out
    }

    fn walk_images(n: &RenderNode, taffy: &Tree, origin: (f32, f32), out: &mut Vec<ImagePlan>) {
        let s = &n.style;
        if s.is_fixed || s.transform.is_some() || s.animation.is_some() || n.tag == "scroll-view" {
            return;
        }
        let Ok(l) = taffy.layout(n.taffy_node) else { return };
        let (x, y) = (origin.0 + l.location.x, origin.1 + l.location.y);
        if n.tag == "image" {
            let mut parts = n.text.splitn(2, '|');
            let src = parts.next().unwrap_or("").trim();
            let mode = parts.next().unwrap_or("scaleToFill");
            let (w, h) = (l.size.width, l.size.height);
            if !src.is_empty() && w > 0.0 && h > 0.0 {
                let radius = crate::renderer::components::get_border_radii_clamped(s, w, h)[0];
                out.push(ImagePlan { src: src.to_string(), mode: mode.to_string(), x, y, w, h, radius });
            }
            return;
        }
        for c in &n.children {
            Self::walk_images(c, taffy, (x, y), out);
        }
    }
}
