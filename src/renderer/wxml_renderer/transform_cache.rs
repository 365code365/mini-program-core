//! 带旋转 / 缩放 / 倾斜的子树：离屏那一张按「内容」缓存，动画每帧只换矩阵、只做仿射贴回。
//!
//! 为什么要它：`@keyframes spin { to { transform: rotate(360deg) } }` 这种转圈（播放页的唱片、加载圈、
//! 旋转的图标）每帧都在动，从前每帧都把整棵子树重新光栅化进离屏再贴回。云听音乐播放页的唱片
//! （径向渐变 + 阴影圈 + 圆角封面图，656×656 物理像素）光栅化一趟占这一帧的一大半。
//! 动画只改了 transform，子树画出来的像素一帧和一帧完全相同 —— 画一次、留着、每帧贴回就够了。
//!
//! 什么时候能用（全部满足才走缓存，否则照旧每帧画）：
//! - 动画只动 transform 与 opacity（成组透明度在贴回那一步乘，离屏里始终按不透明画）：
//!   背景色 / 文字颜色在动的不缓存（键里看不出来）；
//! - 子树是静态的（[`super::scroll_tiles::static_node`]：没有自己的动画 / 过渡 / transform、表单与原生组件、动图）；
//! - 子树里没有正被按着的（点击态画在子树里，键里看不出来；按着的在子树外不算）；
//! - 键相同：节点身份 + 离屏尺寸 + 子树签名（结构、文字、属性、节点身份、相对布局，同分块缓存）。
//!   图片下完 / canvas 画过这类签名看不出来的变化走 `invalidate_offscreen_caches`，整个丢掉。
//!
//! 缓存的那张是**整块**（不按补条带的裁剪收窄）、画的时候拿掉损伤区裁剪：哪一帧要哪一块都能贴。
//! 贴回只采裁剪区里要的像素，结果与每帧现画（收窄过的离屏）逐字节相同 —— 收窄只是少画采不到的像素。
use super::*;

/// 命中缓存（只贴回、没重新光栅化）的次数（测试用：拖进度条时唱片照样走缓存）
static HITS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// 变换子树离屏缓存命中的次数（测试用）
pub fn transform_cache_hits() -> usize {
    HITS.load(std::sync::atomic::Ordering::Relaxed)
}

/// 最多留几张（一页上同时转的东西不会多）
const MAX_ENTRIES: usize = 16;
/// 最多占多少字节（超了先丢最久没用的）
const MAX_BYTES: usize = 24 << 20;

struct Entry {
    key: (u32, u32, u64),
    canvas: Canvas,
    used: u64,
}

#[derive(Default)]
pub(super) struct TransformCache {
    entries: HashMap<NodeId, Entry>,
    tick: u64,
}

impl TransformCache {
    /// 有这一张就记一次「用过」，返回有没有
    fn touch(&mut self, id: NodeId, key: (u32, u32, u64)) -> bool {
        self.tick += 1;
        let tick = self.tick;
        match self.entries.get_mut(&id).filter(|e| e.key == key) {
            Some(e) => {
                e.used = tick;
                true
            }
            None => false,
        }
    }

    /// 缓存里的那张（[`WxmlRenderer::ensure_transform_offscreen`] 之后取）
    pub(super) fn canvas(&self, id: NodeId) -> Option<&Canvas> {
        self.entries.get(&id).map(|e| &e.canvas)
    }

    fn put(&mut self, id: NodeId, key: (u32, u32, u64), canvas: Canvas) {
        self.tick += 1;
        self.entries.insert(id, Entry { key, canvas, used: self.tick });
        let bytes = |m: &HashMap<NodeId, Entry>| m.values().map(|e| e.canvas.pixels().len() * 4).sum::<usize>();
        while self.entries.len() > MAX_ENTRIES || (self.entries.len() > 1 && bytes(&self.entries) > MAX_BYTES) {
            let Some(old) = self.entries.iter().min_by_key(|(_, e)| e.used).map(|(k, _)| *k) else { break };
            self.entries.remove(&old);
        }
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(super) fn buffers(&self) -> impl Iterator<Item = &[Color]> {
        self.entries.values().map(|e| e.canvas.pixels())
    }
}

impl WxmlRenderer {
    /// 这一帧这个 transform 子树能不能用（或写进）缓存。`resolved` 是动画求值后的节点
    pub(super) fn transform_cacheable(&self, node: &RenderNode, resolved: &RenderNode, baked: bool, interaction: &InteractionManager) -> bool {
        if baked || std::env::var_os("MINI_NO_TRANSFORM_CACHE").is_some() {
            return false;
        }
        // 按着的是子树外面的东西（拖进度条时唱片照转）不影响子树画出来的样子：只有子树里有按压态样式
        // （hover-class / :active）的节点时才不缓存。从前按着任何东西都不缓存，拖进度条那 2.5 秒里
        // 唱片每帧重新光栅化（圆角 + 阴影 + 封面），一加上一帧多 6ms（simpleperf：fill_round_rect / blend_mask / memset）
        if !interaction.pressed_visual().is_empty() && Self::subtree_pressable(node) {
            return false;
        }
        let (a, b) = (&node.style, &resolved.style);
        // 动画改了颜色：键里看不出来，不缓存（透明度不算：成组透明度在贴回时乘）
        if a.background_color != b.background_color || a.text_color != b.text_color || b.background_gradient.is_none() && a.background_gradient.is_some() {
            return false;
        }
        node.children.iter().all(super::scroll_tiles::static_node) && !Self::subtree_has_animated_image(node)
    }

    /// 子树里有没有按压态样式（按下时会画成另一个样子）的节点
    fn subtree_pressable(n: &RenderNode) -> bool {
        n.style.pressed_style.is_some() || n.children.iter().any(Self::subtree_pressable)
    }

    /// 缓存键：离屏尺寸 + 子树签名（含节点自己的布局尺寸）
    fn transform_key(taffy: &Tree, node: &RenderNode, (tw, th): (u32, u32)) -> (u32, u32, u64) {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        u64::from(node.taffy_node).hash(&mut h);
        if let Ok(l) = taffy.layout(node.taffy_node) {
            l.size.width.to_bits().hash(&mut h);
            l.size.height.to_bits().hash(&mut h);
        }
        Self::subtree_sig(taffy, node).hash(&mut h);
        (tw, th, h.finish())
    }

    /// 保证缓存里有这棵子树当前内容的那张离屏（没有就整块画一张放进去），之后用 `transform_cache.canvas(id)` 取。
    /// `draw` 把子树画进给它的离屏（局部坐标）
    pub(super) fn ensure_transform_offscreen(
        &mut self,
        taffy: &Tree,
        node: &RenderNode,
        size: (u32, u32),
        draw: impl FnOnce(&mut Self, &mut Canvas),
    ) {
        let key = Self::transform_key(taffy, node, size);
        if self.transform_cache.touch(node.taffy_node, key) {
            HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return;
        }
        let mut off = Canvas::new(size.0, size.1);
        off.clear(Color::TRANSPARENT);
        // 整块画：补条带的损伤区裁剪拿掉（下一帧要的可能是另一块）
        let saved = self.damage_clip.take();
        draw(self, &mut off);
        self.damage_clip = saved;
        self.transform_cache.put(node.taffy_node, key, off);
    }
}
