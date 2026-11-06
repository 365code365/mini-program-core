//! 后台下完 / 解完一张图（`components::image_net`）：只重画用到它的那几块，不再整帧。
//!
//! 从前图一到宿主就整页重画（页面条带 + 覆盖层 + tabBar）。播放页进去的那一秒里封面、背景、
//! 歌单缩略图一张张到，一张一帧整帧：一加 PLY110 上一帧 45~52ms（`engine_frame.rs` 那条 `mark_full`），
//! 正好叠在转场动画上 —— 「进入播放页很卡」的一半。
//!
//! 数据没变，布局不变，变的只是用到这张图的节点画出来的像素。按当前布局树找出这些节点画在哪：
//! - 普通流里的 `<image>`（`text` 是 `src|mode`）/ `<video poster>`：它的框（连阴影）；
//! - 在 scroll-view / swiper 里（能滚的那种）：里面的布局坐标没扣滚动偏移，重画整个盒子（与失效分析同一取舍）；
//!   `clip=false` 的说不准画到哪 → 整帧；
//! - 在带 transform / CSS 动画的子树里：整段动画最多画到的范围（[`WxmlRenderer::anim_extent`]）；
//! - 在覆盖层（fixed）里：覆盖层整层重画；
//! - 远程图 `mode="widthFix"` / `"heightFix"`：图的比例决定框的大小，**要重排**（[`ImageArrival::Relayout`]）。
//!   从前只是整帧重画、布局缓存照旧命中（数据没变），远程 widthFix 图下完了也一直按 320×240 的默认框画，
//!   直到下一次 setData 整棵重建；增量重建还会把那棵没变的子树原样留着，连那一次也不一定重排。
//!   本地图的比例建树时就从文件头读到了（image/intrinsic.rs），不用重排。
//!
//! 页面树里没有节点用的 key（被盖住那一页发起的下载、宿主浮层的图、tabBar 图标）报个数（`missing`），由宿主判。
use super::invalidate::DiffCtx;
use super::*;

/// 这一批图到了之后该重画多大
#[derive(Debug, Clone, PartialEq)]
pub enum ImageArrival {
    /// 只重画这几块：页面画布上的并集（物理像素，画布坐标；没有就是 None），覆盖层要不要整层重画。
    /// `missing`：这批里有几张页面树里没有节点用（被盖住那一页的、宿主浮层的、tabBar 的），由宿主判
    Rects { page: Option<GeoRect>, fixed: bool, missing: usize },
    /// 有 widthFix / heightFix 的图：布局要重排（之后整帧）
    Relayout,
    /// 说不准画在哪（原因诊断用）
    Full(String),
}

#[derive(Default)]
struct Acc {
    page: Option<GeoRect>,
    fixed: bool,
    relayout: bool,
    full: Option<String>,
    found: std::collections::HashSet<String>,
}

impl Acc {
    fn add(&mut self, r: GeoRect) {
        if r.width <= 0.0 || r.height <= 0.0 {
            return;
        }
        self.page = Some(match self.page {
            None => r,
            Some(u) => {
                let (x0, y0) = (u.x.min(r.x), u.y.min(r.y));
                let (x1, y1) = ((u.x + u.width).max(r.x + r.width), (u.y + u.height).max(r.y + r.height));
                GeoRect::new(x0, y0, x1 - x0, y1 - y0)
            }
        });
    }
}

/// 节点用到的、在这批里的那张图的 key（`<image>` 的 src、`<video>` 的 poster），以及要不要重排
fn uses<'a>(n: &RenderNode, keys: &'a std::collections::HashSet<String>) -> Option<(&'a String, bool)> {
    let hit = |s: &str| keys.get(s).or_else(|| keys.get(s.trim()));
    match n.tag.as_str() {
        "image" => {
            let mut parts = n.text.split('|');
            let src = parts.next().unwrap_or("");
            let mode = parts.next().unwrap_or("").trim();
            // 本地图的比例建树时就从文件头读到了（image/intrinsic.rs），框早就对；只有远程图下完才知道
            let remote = src.trim().starts_with("http://") || src.trim().starts_with("https://");
            hit(src).map(|k| (k, remote && matches!(mode, "widthFix" | "heightFix")))
        }
        "video" => n.attrs.get("poster").and_then(|p| hit(p)).map(|k| (k, false)),
        _ => None,
    }
}

impl WxmlRenderer {
    /// 这批 key（`image_net::take_arrived`：远程是地址本身，本地大图是 `local:` + src）用在页面哪几块
    pub fn image_arrival_damage(&mut self, arrived: &[String], viewport: Option<(f32, f32)>) -> ImageArrival {
        let keys: std::collections::HashSet<String> =
            arrived.iter().map(|k| k.strip_prefix("local:").unwrap_or(k).to_string()).collect();
        let Some(cache) = self.cache.as_ref() else { return ImageArrival::Full("还没布局过".into()) };
        let empty = HashMap::new();
        let ctx = DiffCtx {
            moved: &empty,
            union: None,
            parts: Vec::new(),
            fixed_changed: false,
            bail: false,
            bail_why: String::new(),
            grow: false,
            cut: None,
            positional: Default::default(),
            sheet: &self.stylesheet,
            units: (self.screen_width, self.scale_factor),
        };
        let mut acc = Acc::default();
        for n in &cache.render_nodes {
            Self::arrival_walk(&cache.taffy, n, (0.0, 0.0), false, &keys, &ctx, &mut acc);
        }
        // 重排先判：它之后本来就整帧，而说不准画在哪的那种整帧不重排
        if acc.relayout {
            return ImageArrival::Relayout;
        }
        if let Some(why) = acc.full {
            return ImageArrival::Full(why);
        }
        let missing = keys.iter().filter(|k| !acc.found.contains(*k)).count();
        let page = acc.page.map(|r| self.sticky_widen(r, viewport));
        ImageArrival::Rects { page, fixed: acc.fixed, missing }
    }

    #[allow(clippy::too_many_arguments)]
    fn arrival_walk(
        t: &Tree,
        n: &RenderNode,
        (px, py): (f32, f32),
        in_fixed: bool,
        keys: &std::collections::HashSet<String>,
        ctx: &DiffCtx,
        acc: &mut Acc,
    ) {
        // 已经要整帧了也走完：后面还可能有要重排的
        let Ok(l) = t.layout(n.taffy_node) else {
            acc.full = Some(format!("<{}> 取不到布局", n.tag));
            return;
        };
        let (x, y) = (px + l.location.x, py + l.location.y);
        let fixed = in_fixed || n.style.is_fixed;
        let rect = GeoRect::new(x, y, l.size.width, l.size.height);
        // 覆盖层整层重画，里面画在哪不用算；只看要不要重排
        if fixed {
            let mut hits = Vec::new();
            Self::arrival_hits(n, keys, &mut hits);
            for (k, relayout) in hits {
                acc.found.insert(k.clone());
                acc.fixed = true;
                acc.relayout |= relayout;
            }
            return;
        }
        let scrolls = matches!(n.tag.as_str(), "scroll-view" | "swiper") && {
            n.tag == "swiper"
                || n.children.iter().any(|c| {
                    t.layout(c.taffy_node).is_ok_and(|cl| {
                        cl.location.y + cl.size.height > l.size.height + 0.5 || cl.location.x + cl.size.width > l.size.width + 0.5
                    })
                })
        };
        let moving = n.style.transform.is_some() || n.style.animation.is_some();
        if scrolls || moving {
            let mut hits = Vec::new();
            Self::arrival_hits(n, keys, &mut hits);
            if hits.is_empty() {
                return;
            }
            for (k, relayout) in hits {
                acc.found.insert(k.clone());
                acc.relayout |= relayout;
            }
            if scrolls {
                let clips = if n.tag == "swiper" {
                    crate::renderer::components::SwiperComponent::clips(n)
                } else {
                    super::scroll_views::clips(&n.attrs)
                };
                if !clips {
                    acc.full = Some(format!("<{} clip=false> 里的图", n.tag));
                    return;
                }
                acc.add(super::invalidate_grow::inflate(rect, &n.style));
            } else {
                match Self::anim_extent(t, n, (px, py), ctx) {
                    Some(r) => acc.add(r),
                    None => acc.full = Some(format!("<{}> 变换 / 动画画到哪算不准", n.tag)),
                }
            }
            return;
        }
        if let Some((k, relayout)) = uses(n, keys) {
            acc.found.insert(k.clone());
            acc.relayout |= relayout;
            acc.add(super::invalidate_grow::inflate(rect, &n.style));
        }
        for c in &n.children {
            Self::arrival_walk(t, c, (x, y), false, keys, ctx, acc);
        }
    }

    /// 布局树里有没有节点用到这批图里的任何一张（tabBar 渲染器用）
    pub fn uses_any_image(&self, arrived: &[String]) -> bool {
        let keys: std::collections::HashSet<String> =
            arrived.iter().map(|k| k.strip_prefix("local:").unwrap_or(k).to_string()).collect();
        let Some(cache) = self.cache.as_ref() else { return false };
        let mut hits = Vec::new();
        for n in &cache.render_nodes {
            Self::arrival_hits(n, &keys, &mut hits);
        }
        !hits.is_empty()
    }

    /// 子树里用到这批图的节点（key，要不要重排）
    fn arrival_hits<'a>(n: &RenderNode, keys: &'a std::collections::HashSet<String>, out: &mut Vec<(&'a String, bool)>) {
        if let Some(h) = uses(n, keys) {
            out.push(h);
        }
        for c in &n.children {
            Self::arrival_hits(c, keys, out);
        }
    }
}
