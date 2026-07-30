//! **标签 → 组件行为的唯一登记点。**
//!
//! ## 为什么要有这张表
//! 「一个标签怎么建树、怎么画、是不是叶子、绘制耗时算到哪一类」这四件事从前分散在
//! **两个文件的四处 match** 里：
//!
//! | 分散在哪 | 内容 |
//! |---|---|
//! | `wxml_renderer/layout.rs` | 21 条 `build` 分支 |
//! | `wxml_renderer/layout.rs` | `is_leaf_component` 的 13 个标签清单 |
//! | `wxml_renderer/draw.rs` | 10 条「绘制归因名」分支 |
//! | `wxml_renderer/draw.rs` | 19 条 `draw` 分支 |
//!
//! 加一个组件要改三到四处，漏一处的后果各不相同：漏 build → 当成 `view`；
//! 漏 draw → 只画背景；漏叶子清单 → 子节点被重复绘制；漏归因名 → 落进「其它」。
//! 这种「同一件事登记在多处」是这个仓库反复出问题的形状（此前还有一份**死的**
//! 第二套 build 分派表，21 条分支，谁在那里加组件都不生效）。
//!
//! 现在一个标签一行，四件事写在一起；新增组件只动这张表。
//!
//! ## 绘制签名为什么要打包成 [`DrawCtx`]
//! 各组件的 `draw` 原本是 6~8 个位置参数，有的要字体、有的不要，于是分派处只能靠
//! 一长串 match 逐个转发。打包成一个上下文之后，表里每一行的形状就一样了 ——
//! 代价是每行一个薄适配闭包（非捕获闭包会被强制转成 `fn` 指针，无额外开销）。

use super::*;
use crate::parser::wxml::WxmlNode;
use crate::text::TextRenderer;
use crate::Canvas;

/// 画一个组件要用到的全部东西（设备像素坐标）
pub struct DrawCtx<'a> {
    pub node: &'a RenderNode,
    pub canvas: &'a mut Canvas,
    /// 需要度量/绘制文字的组件才用得上
    pub text: Option<&'a TextRenderer>,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// 设备像素比
    pub sf: f32,
}

/// 一个标签的全部行为
pub struct TagSpec {
    /// 这条规格覆盖的标签。`input`/`textarea` 共用一份实现，所以是数组。
    pub tags: &'static [&'static str],
    /// 绘制耗时归因的分类名（`MINI_DRAW_LOG=1`）。没单独分类的落 `"其它"`。
    pub profile: &'static str,
    /// 是否是**叶子**：叶子把子内容自己合成掉（如 `<button>文字</button>`），
    /// 渲染器不再单独遍历它的子节点。
    pub leaf: bool,
    /// 是否有**交互状态**（勾选 / 拨动 / 滑块值 / 输入文本）。绘制前要把状态落到
    /// 本帧节点上（见 `wxml_renderer::interactive`），也决定了要不要为它做浅拷贝。
    pub stateful: bool,
    /// 是否**自己裁剪子树**：滚动容器与轮播的内容超出部分不参与外层的溢出累计
    /// （语义与 CSS 的可滚溢出区一致）。
    pub clips: bool,
    /// 建树：WXML 节点 → 渲染节点（挂 taffy 节点、算样式）
    pub build: fn(&WxmlNode, &mut ComponentContext) -> Option<RenderNode>,
    /// 绘制自身（子节点由渲染器递归）
    pub draw: fn(&mut DrawCtx),
}

/// 未登记标签的兜底：按 `view` 处理（微信里未知标签也是块级容器）
const FALLBACK: TagSpec = TagSpec {
    tags: &[],
    profile: "其它",
    leaf: false,
    stateful: false,
    clips: false,
    build: |n, c| ViewComponent::build(n, c),
    draw: |d| ViewComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
};

/// 一个标签一行。**顺序无关**，查找按标签名。
pub const TAGS: &[TagSpec] = &[
    // ── 容器 ──
    TagSpec {
        // `""` 是模板展开出的匿名容器（自定义组件的宿主节点）
        tags: &["view", ""],
        profile: "view",
        leaf: false,
        stateful: false,
        clips: false,
        build: |n, c| ViewComponent::build(n, c),
        draw: |d| ViewComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["block"],
        profile: "其它",
        leaf: false,
        stateful: false,
        clips: false,
        build: |n, c| ViewComponent::build(n, c),
        draw: |d| ViewComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["scroll-view"],
        profile: "scroll-view",
        leaf: false,
        stateful: false,
        clips: true,
        build: |n, c| ViewComponent::build(n, c),
        draw: |d| ViewComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["swiper"],
        profile: "swiper",
        leaf: false,
        stateful: false,
        clips: true,
        build: |n, c| SwiperComponent::build(n, c),
        // 只画背景：子项与指示点由 `draw_swiper_container` 按当前页与过渡进度画
        draw: |d| draw_background(d.canvas, &d.node.style, d.x, d.y, d.w, d.h),
    },
    TagSpec {
        tags: &["swiper-item"],
        profile: "其它",
        leaf: false,
        stateful: false,
        clips: false,
        build: |n, c| SwiperItemComponent::build(n, c),
        draw: |d| ViewComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    // ── 文本与媒体 ──
    TagSpec {
        // `#text` 是裸文本节点（`<view>abc</view>` 里的 abc），不经 build 分派
        tags: &["text", "#text"],
        profile: "text",
        leaf: true,
        stateful: false,
        clips: false,
        build: |n, c| TextComponent::build(n, c),
        draw: |d| TextComponent::draw(d.node, d.canvas, d.text, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["rich-text"],
        profile: "其它",
        leaf: false,
        stateful: false,
        clips: false,
        build: |n, c| RichTextComponent::build(n, c),
        draw: |d| RichTextComponent::draw(d.node, d.canvas, d.text, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["icon"],
        profile: "其它",
        leaf: true,
        stateful: false,
        clips: false,
        build: |n, c| IconComponent::build(n, c),
        draw: |d| IconComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["image"],
        profile: "image",
        leaf: true,
        stateful: false,
        clips: false,
        build: |n, c| ImageComponent::build(n, c),
        draw: |d| ImageComponent::draw(d.node, d.canvas, d.text, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["video"],
        profile: "video",
        leaf: true,
        stateful: false,
        clips: false,
        build: |n, c| VideoComponent::build(n, c),
        draw: |d| VideoComponent::draw(d.node, d.canvas, d.text, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["canvas"],
        profile: "canvas",
        leaf: true,
        stateful: false,
        clips: false,
        build: |n, c| CanvasComponent::build(n, c),
        draw: |d| CanvasComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    // ── 表单 ──
    TagSpec {
        tags: &["button"],
        profile: "button",
        leaf: true,
        stateful: false,
        clips: false,
        build: |n, c| ButtonComponent::build(n, c),
        draw: |d| ButtonComponent::draw(d.node, d.canvas, d.text, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["input", "textarea"],
        profile: "input",
        leaf: true,
        stateful: true,
        clips: false,
        build: |n, c| InputComponent::build(n, c),
        draw: |d| InputComponent::draw(d.node, d.canvas, d.text, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["switch"],
        profile: "其它",
        leaf: true,
        stateful: true,
        clips: false,
        build: |n, c| SwitchComponent::build(n, c),
        draw: |d| SwitchComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["slider"],
        profile: "其它",
        leaf: true,
        stateful: true,
        clips: false,
        build: |n, c| SliderComponent::build(n, c),
        draw: |d| SliderComponent::draw(d.node, d.canvas, d.text, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["progress"],
        profile: "其它",
        leaf: true,
        stateful: false,
        clips: false,
        build: |n, c| ProgressComponent::build(n, c),
        draw: |d| ProgressComponent::draw(d.node, d.canvas, d.text, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["checkbox"],
        profile: "其它",
        leaf: true,
        stateful: true,
        clips: false,
        build: |n, c| CheckboxComponent::build(n, c),
        draw: |d| CheckboxComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["checkbox-group"],
        profile: "其它",
        leaf: false,
        stateful: false,
        clips: false,
        build: |n, c| CheckboxGroupComponent::build(n, c),
        draw: |d| CheckboxGroupComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["radio"],
        profile: "其它",
        leaf: true,
        stateful: true,
        clips: false,
        build: |n, c| RadioComponent::build(n, c),
        draw: |d| RadioComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["radio-group"],
        profile: "其它",
        leaf: false,
        stateful: false,
        clips: false,
        build: |n, c| RadioGroupComponent::build(n, c),
        draw: |d| RadioGroupComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["picker"],
        profile: "其它",
        // picker 画的是「触发视图」（页面里那行「当前选择：xxx」），子节点要照常渲染；
        // 底部选择面板是宿主的浮层，见 host::picker_sheet
        leaf: false,
        stateful: false,
        clips: false,
        build: |n, c| PickerComponent::build(n, c),
        draw: |d| PickerComponent::draw(d.node, d.canvas, d.text, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["picker-view"],
        profile: "其它",
        leaf: false,
        stateful: false,
        clips: false,
        build: |n, c| PickerViewComponent::build(n, c),
        draw: |d| PickerViewComponent::draw(d.node, d.canvas, d.text, d.x, d.y, d.w, d.h, d.sf),
    },
    TagSpec {
        tags: &["picker-view-column"],
        profile: "其它",
        leaf: true,
        stateful: false,
        clips: false,
        build: |n, c| PickerViewColumnComponent::build(n, c),
        draw: |d| ViewComponent::draw(d.node, d.canvas, d.x, d.y, d.w, d.h, d.sf),
    },
];

/// 查一个标签的规格。未登记的标签按 `view` 兜底。
///
/// 表只有二十多行，线性扫比建哈希表快（也不用惰性初始化的锁）。
pub fn spec_for(tag: &str) -> &'static TagSpec {
    TAGS.iter()
        .find(|s| s.tags.contains(&tag))
        .unwrap_or(&FALLBACK)
}

/// 已登记的标签总数（文档与测试用：README 说「24 个标签」得有出处）
pub fn registered_tag_count() -> usize {
    TAGS.iter().map(|s| s.tags.len()).sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 每个标签只登记一次() {
        let mut seen = std::collections::HashSet::new();
        for spec in TAGS {
            for tag in spec.tags {
                assert!(seen.insert(*tag), "标签 {tag} 被登记了两次");
            }
        }
    }

    #[test]
    fn 未知标签按_view_兜底() {
        let s = spec_for("web-view");
        assert_eq!(s.profile, "其它");
        assert!(!s.leaf, "兜底是容器，子节点要照常渲染");
    }

    #[test]
    fn 表里覆盖了文档声称的标签数() {
        // README / 支持范围表写的是 24 个标签；这里多出的两个是内部用的
        // `#text`（裸文本节点）与 `""`（模板展开的匿名容器）。
        assert_eq!(registered_tag_count(), 26, "标签数变了就同步一下文档");
    }
}
