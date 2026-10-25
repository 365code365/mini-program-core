//! 嵌在容器里的纵向 scroll-view：内容按固定行数分块缓存，滚动时只把块贴回页面画布。
//!
//! 为什么要它：顶层 scroll-view 有整段离屏缓存（draw_interactive.rs），嵌在 `<view>` 里的（「定高
//! scroll-view 当页面滚、底下压一条输入栏」这种很常见的写法）却每帧把可见子节点重新光栅化一遍。
//! 兴趣圈帖子页（诊断 20261025-100253-98de9a）的 scroll-view 占屏 93%，一加 PLY110 上一帧光画就 10ms
//! （三张 135° 渐变图在视口里时 12ms），90Hz 一半的帧超预算。
//!
//! 为什么不直接套用顶层那份整段缓存：整段按内容高分配（这一页 1270×7000 行 ≈ 36MB），而且它走的是
//! 退化的绘制路径（没有按压态）。这里：
//! - **分块**：第 k 块是内容的 `[k*TILE_ROWS, (k+1)*TILE_ROWS)` 行，只留视口上下各一块，约 6 块 ≈ 15MB；
//!   块的位置只取决于 k，开 / 关局部重画的两个引擎画出来逐字节相同（`sdk_damage_equivalence` 的前提）；
//! - **同一条绘制路径**：块里用 `draw_child_with_interaction` 画（opacity 组、文字、圆角与逐帧画时同一份），
//!   命中区与事件绑定画块时写进一次性容器丢掉，每帧按贴出来的位置重新登记（与顶层缓存同一个函数）；
//! - **按着的时候不用块**：块里画不出按压态，点击态在 scroll-view 里亮着时这一帧照旧逐帧画；
//! - **只接得住静态内容**：子树里有动画 / 过渡 / transform、有状态的表单组件、视频地图画布、
//!   swiper / picker-view / 嵌套 scroll-view / 动图的，一律照旧逐帧画（这些要么每帧在变，要么状态不在节点上）。
//!
//! 贴块按整数行：滚动偏移在画布上四舍五入到物理像素（逐帧画那条路对这种 scroll-view 也按同一个取整，
//! 按着 / 松开两条路之间不会跳半个像素）。
use super::*;
use crate::ui::scroll_cache::{TileSet, TILE_ROWS};

impl WxmlRenderer {
    /// 这个 scroll-view 能不能走分块缓存（只看子树结构；按没按着、底色是不是纯色另判）
    pub(super) fn tile_eligible(&self, node: &RenderNode) -> bool {
        if std::env::var_os("MINI_NO_SCROLL_TILES").is_some() || self.drawing_offscreen {
            return false;
        }
        fn ok(n: &RenderNode) -> bool {
            const LIVE_TAGS: &[&str] = &[
                "scroll-view", "swiper", "picker-view", "video", "map", "canvas", "input", "textarea", "editor",
                "checkbox", "radio", "switch", "slider", "progress", "movable-area", "movable-view", "live-player",
                "live-pusher", "camera", "web-view", "voip-room", "rich-text",
            ];
            if LIVE_TAGS.contains(&n.tag.as_str()) {
                return false;
            }
            // 动图在第一次画（解码）之前认不出来（`subtree_has_animated_image` 只认解码过的）：按扩展名先挡住。
            // 不挡的话第一趟画进块里，帧位置记成块坐标，之后换帧画到别处（gallery 35_gif_frame3 / 4 踩过）
            if n.tag == "image" {
                let src = n.attrs.get("src").map(|s| s.to_ascii_lowercase()).unwrap_or_default();
                if [".gif", ".webp", ".apng"].iter().any(|ext| src.split(['?', '#']).next().unwrap_or("").ends_with(ext)) {
                    return false;
                }
            }
            let s = &n.style;
            if s.animation.is_some() || s.transform.is_some() || s.transition.is_some() || s.is_fixed {
                return false;
            }
            if s.pressed_style.as_ref().is_some_and(|p| p.transform.is_some()) {
                return false;
            }
            n.children.iter().all(ok)
        }
        node.children.iter().all(ok) && !Self::subtree_has_animated_image(node)
    }

    /// scroll-view 里有元素正画成点击态（含 `hover-stay-time` 里的）：这一帧逐帧画。
    /// 画到这里时这一帧的命中区多半还没登记（元素表每帧清空重建，scroll-view 里的在下面才登记），
    /// 查不到包围盒的按「在里面」算 —— 宁可多逐帧画一帧，不能把按压态画丢
    fn pressing_inside(interaction: &InteractionManager, sv: &GeoRect) -> bool {
        interaction.pressed_visual().iter().any(|id| match interaction.element_bounds(id) {
            None => true,
            Some((b, fixed)) => {
                !fixed && b.x < sv.x + sv.width && b.x + b.width > sv.x && b.y < sv.y + sv.height && b.y + b.height > sv.y
            }
        })
    }

    /// 底下这块是不是一种不透明的纯色（只看当前裁剪区里的那段：区外是上一帧留下的像素）
    fn uniform_backdrop(canvas: &Canvas, x: f32, y: f32, w: f32, h: f32) -> Option<Color> {
        let (bx0, bx1, by0, by1) = canvas.draw_bounds();
        let (x0, x1) = ((x.ceil() as i32).max(bx0), ((x + w).floor() as i32).min(bx1));
        let (y0, y1) = ((y.ceil() as i32).max(by0), ((y + h).floor() as i32).min(by1));
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let px = canvas.pixels();
        let cw = canvas.width() as usize;
        let c = px[y0 as usize * cw + x0 as usize];
        if c.a != 255 {
            return None;
        }
        for yy in y0..y1 {
            let row = &px[yy as usize * cw + x0 as usize..yy as usize * cw + x1 as usize];
            if row.iter().any(|p| *p != c) {
                return None;
            }
        }
        Some(c)
    }

    /// 走分块画这个 scroll-view。返回 false 表示这一帧没走（调用方照旧逐帧画）
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_scroll_view_tiled(
        &mut self,
        canvas: &mut Canvas,
        taffy: &Tree,
        node: &RenderNode,
        (x, y, w, h): (f32, f32, f32, f32),
        component_id: &str,
        logical_bounds: GeoRect,
        interaction: &mut InteractionManager,
    ) -> bool {
        let sf = self.scale_factor;
        let pos = interaction.get_scroll_controller(component_id).map(|c| c.get_position()).unwrap_or(0.0);
        // 顶部拉出空白：开了下拉刷新的空白里要画刷新区，照旧逐帧画。没开的那段空白什么都不画（露出底色），
        // 块盖不到负的行，底色本来就在 —— 与逐帧画一样。从前橡皮筋一拉就逐帧画，一加上那几帧 12~14ms
        let refresher = super::scroll_views::bool_attr(&node.attrs, "refresher-enabled");
        if (pos < 0.0 && refresher) || w < 1.0 || h < 1.0 || Self::pressing_inside(interaction, &logical_bounds) {
            return false;
        }
        let mut content_h = 0.0f32;
        for c in &node.children {
            if let Ok(l) = taffy.layout(c.taffy_node) {
                content_h = content_h.max(l.location.y + l.size.height);
            }
        }
        let top = (pos * sf).round() as i64;
        let (xi, yi) = (x.floor(), y.floor());
        let (fx, fy) = (x - xi, y - yi);
        let wpx = w.ceil() as u32;
        let key = (wpx, fx.to_bits(), fy.to_bits(), content_h.ceil() as u32);
        let data_gen = self.scroll_cache.data_gen;
        let mut reuse = self.scroll_cache.tiles.get(component_id).is_some_and(|t| t.key == key);
        // 数据变过：核对子树签名，没变就接着用这些块（只在数据变了的那一帧算一次）
        let mut sig = None;
        if reuse && self.scroll_cache.tiles.get(component_id).is_some_and(|t| t.data_gen != data_gen) {
            let s = Self::subtree_sig(taffy, node);
            sig = Some(s);
            match self.scroll_cache.tiles.get_mut(component_id) {
                Some(t) if t.sig == s => t.data_gen = data_gen,
                _ => reuse = false,
            }
        }
        if !reuse {
            // 新建：先确认底下是纯色（块先铺成它，再画内容 —— 与直接画在页面上同一个合成顺序）
            let Some(bg) = Self::uniform_backdrop(canvas, x, y, w, h) else {
                self.scroll_cache.tiles.remove(component_id);
                return false;
            };
            let sig = sig.unwrap_or_else(|| Self::subtree_sig(taffy, node));
            self.scroll_cache.tiles.insert(component_id.to_string(), TileSet { key, bg, tiles: Default::default(), sig, data_gen });
        }
        let t = TILE_ROWS as i64;
        let (k0, k1) = (top.div_euclid(t), (top + h.ceil() as i64).div_euclid(t));
        // 视口上下各留一块，再远的放掉（内存只随视口高走，不随内容高走）
        if let Some(set) = self.scroll_cache.tiles.get_mut(component_id) {
            set.tiles.retain(|k, _| (*k as i64) >= k0 - 1 && (*k as i64) <= k1 + 1);
        }
        let text_color = node.style.text_color.unwrap_or(Color::BLACK);
        for k in k0.max(0)..=k1 {
            let k = k as u32;
            let have = self.scroll_cache.tiles.get(component_id).is_some_and(|s| s.tiles.contains_key(&k));
            if !have {
                let bg = self.scroll_cache.tiles[component_id].bg;
                let tile = self.render_tile(taffy, node, k, (wpx, fx, fy), bg, text_color, (xi, yi, top));
                // 底色不透明、内容画在上面照样不透明；万一有不透明度没盖住的（理论上不会），贴回去时走混合
                let opaque = tile.pixels().iter().all(|p| p.a == 255);
                if let Some(set) = self.scroll_cache.tiles.get_mut(component_id) {
                    set.tiles.insert(k, (tile, opaque));
                }
            }
        }
        // 贴：块的第 r 行（内容第 k*T + r 行）落在页面画布的 yi + k*T + r - top 行
        canvas.save();
        canvas.clip_rect(GeoRect::new(x, y, w, h));
        if let Some(set) = self.scroll_cache.tiles.get(component_id) {
            for k in k0.max(0)..=k1 {
                if let Some((tile, opaque)) = set.tiles.get(&(k as u32)) {
                    let dy = (yi as i64 + k * t - top) as i32;
                    if *opaque {
                        canvas.copy_opaque_pixels(xi as i32, dy, tile.pixels(), tile.width() as usize);
                    } else {
                        canvas.blend_pixels(xi as i32, dy, tile.pixels(), tile.width() as usize);
                    }
                }
            }
        }
        canvas.restore();
        // 命中区与事件绑定：按贴出来的位置登记（与顶层缓存同一个函数）
        let oy = y - top as f32;
        for child in &node.children {
            self.register_child_interactions(taffy, child, x, oy, text_color, interaction, logical_bounds.y, h / sf);
        }
        true
    }

    /// scroll-view 子树画出来取决于的东西的签名：标签、文字、属性（与顺序无关）、节点身份、相对布局。
    /// 整棵重建（关了增量布局、第一次布局）时节点全是新的，签名必变，块重画 —— 宁可多画不能画错
    fn subtree_sig(taffy: &Tree, node: &RenderNode) -> u64 {
        use std::hash::{Hash, Hasher};
        fn walk(taffy: &Tree, n: &RenderNode, h: &mut std::collections::hash_map::DefaultHasher) {
            n.tag.hash(h);
            n.text.hash(h);
            let mut attrs: u64 = 0;
            for (k, v) in &n.attrs {
                let mut e = std::collections::hash_map::DefaultHasher::new();
                k.hash(&mut e);
                v.hash(&mut e);
                attrs = attrs.wrapping_add(e.finish());
            }
            attrs.hash(h);
            // 节点身份：增量布局只原样留下「样式与布局都没变」的子树（继承来的颜色 / 字号变了它会重建），
            // 重建的节点是新的 taffy 节点 —— 所以计算样式变没变看节点换没换就够
            u64::from(n.taffy_node).hash(h);
            if let Ok(l) = taffy.layout(n.taffy_node) {
                for v in [l.location.x, l.location.y, l.size.width, l.size.height] {
                    v.to_bits().hash(h);
                }
            }
            n.children.len().hash(h);
            for c in &n.children {
                walk(taffy, c, h);
            }
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for c in &node.children {
            walk(taffy, c, &mut h);
        }
        h.finish()
    }

    /// 画第 k 块：内容第 `k*TILE_ROWS` 行起的 `TILE_ROWS` 行，铺底色后用逐帧画的同一条路径画
    #[allow(clippy::too_many_arguments)]
    fn render_tile(
        &mut self,
        taffy: &Tree,
        node: &RenderNode,
        k: u32,
        (wpx, fx, fy): (u32, f32, f32),
        bg: Color,
        text_color: Color,
        (xi, yi, top): (f32, f32, i64),
    ) -> Canvas {
        let t = TILE_ROWS as f32;
        let row0 = k as f32 * t;
        let mut tile = Canvas::new(wpx, TILE_ROWS);
        tile.clear(bg);
        let sf = self.scale_factor.max(0.001);
        // 损伤区与视口裁剪都按页面坐标判，块里是块的局部坐标：画块时换成「整块都要画」
        let saved = (self.damage_clip.take(), self.binding_shift);
        // 块坐标 → 页面坐标（只影响这一趟登记出来的 id，登记本身写进一次性容器）
        self.binding_shift = (saved.1 .0 + xi / sf, saved.1 .1 + (yi + row0 - top as f32) / sf);
        let mut scratch = InteractionManager::new();
        let bindings_before = self.event_bindings.len();
        for child in &node.children {
            let Ok(l) = taffy.layout(child.taffy_node) else { continue };
            if l.location.y + l.size.height < row0 || l.location.y > row0 + t {
                continue;
            }
            self.draw_child_with_interaction(&mut tile, taffy, child, fx, fy - row0, text_color, &mut scratch, 0.0, t);
        }
        self.event_bindings.truncate(bindings_before);
        self.damage_clip = saved.0;
        self.binding_shift = saved.1;
        tile
    }
}
