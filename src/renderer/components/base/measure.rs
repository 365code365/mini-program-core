//! taffy 的文本度量回调与 min-content 宽度
//!
//! 由 `base/mod.rs` 组合。**纯搬迁**：从 1925 行的 base.rs 按职责切开，一行代码没改。
use super::*;

/// taffy 文本度量：给定已知尺寸与可用空间，按 CSS 语义算出文本盒尺寸。
///
/// - 两维都已知：直接返回。
/// - 宽度未知：按 available 决定（Definite=定宽换行；MaxContent=单行内容宽；MinContent=最小单元宽）。

/// - 高度按解析出的行数 * 行高 + 上下内边距。
pub fn measure_text_node(
    known: taffy::geometry::Size<Option<f32>>,
    available: taffy::geometry::Size<taffy::style::AvailableSpace>,
    tm: &TextMeasure,
    tr: Option<&crate::text::TextRenderer>,
) -> taffy::geometry::Size<f32> {
    use taffy::style::AvailableSpace;
    // 指定了 font-family 就用那一族的字体来数换行：与绘制端同一把尺子
    let family = crate::text_family::renderer_for_family(tm.font_family.as_deref());
    let tr = family.as_deref().or(tr);
    let pad_w = tm.pad_l + tm.pad_r;
    let pad_h = tm.pad_t + tm.pad_b;
    let content_w = tm.max_line_width + pad_w;

    // min-content 按定义不可能大于 max-content，这里必须夹一次：
    // `min_unit_width` 垫了「至少一个 em」的底（断行按整字切，可用宽再小也要放得下
    // 一个字），而窄字符的 advance 比 1em 小得多 —— 宋体的 `+` 在 36px 下只有 24。
    // 不夹的话单个 `+` 的文本盒被下限撑到 36：盒子被 `justify-content:center`
    // 居中了，里面的字形却偏左 3px，圆形按钮里的加号看着没对准圆心
    // （商城列表的 `.add` 就是这么歪的）。
    let min_w = (tm.min_unit_width + pad_w).min(content_w);

    // 解析可用宽度 → 内容区宽
    let box_w = match known.width {
        Some(w) => w,
        None => match available.width {
            AvailableSpace::Definite(w) => w.min(content_w).max(min_w),
            AvailableSpace::MaxContent => content_w,
            AvailableSpace::MinContent => min_w,
        },
    };
    // 诊断：`MINI_TEXT_PROBE=<原文>` 打印这段文字的盒宽是怎么定出来的。
    // 度量是热路径，环境变量只读一次（`min-content` 意外大于 `max-content` 这类问题
    // 光看渲染结果是看不出来的 —— 圆按钮里的加号偏左就是它查出来的）。
    {
        static PROBE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
        if PROBE.get_or_init(|| std::env::var("MINI_TEXT_PROBE").ok()).as_deref() == Some(&tm.text) {
            eprintln!(
                "🔎 度量 {:?}: known={:?} avail={:?} → box_w={:.2}（max_line={:.2} min_unit={:.2} pad={:.2} font_px={:.1}）",
                tm.text, known.width, available.width, box_w, tm.max_line_width, tm.min_unit_width, pad_w, tm.font_px
            );
        }
    }
    let inner_w = (box_w - pad_w).max(1.0);

    // 行数：nowrap 只按显式换行；否则按内容宽/可用宽估算，再取真实度量
    let lines = if tm.nowrap {
        tm.min_lines.max(1)
    } else {
        let wrap = if let Some(tr) = tr {
            count_wrapped_text_lines(tr, &tm.text, inner_w, tm.font_px, tm.letter_spacing_px, tm.bold)
        } else if inner_w + WRAP_TOLERANCE_PX < tm.max_line_width {
            (tm.max_line_width / inner_w).ceil() as usize
        } else {
            1
        };
        tm.min_lines.max(wrap).max(1)
    };

    let height = known
        .height
        .unwrap_or(lines as f32 * tm.line_height_px + pad_h);
    taffy::geometry::Size { width: box_w, height }
}

/// 文本的最小内容宽（CSS min-content）：最宽的不可拆分单元。
/// CJK 逐字可断，故为单字宽；连续 ASCII 视作整词不可断，取最长词宽。
pub fn min_unit_width(
    text: &str,
    tr: Option<&crate::text::TextRenderer>,
    font_px: f32,
    letter_spacing_px: f32,
    bold: bool,
) -> f32 {
    let measure = |s: &str| -> f32 {
        if let Some(tr) = tr {
            tr.measure_text_weighted(s, font_px, letter_spacing_px, bold)
        } else {
            s.chars()
                .map(|c| if c.is_ascii() { font_px * 0.62 } else { font_px } + letter_spacing_px)
                .sum()
        }
    };
    let mut widest = 0.0f32;
    let mut word = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            word.push(ch);
        } else {
            if !word.is_empty() {
                widest = widest.max(measure(&word));
                word.clear();
            }
            if !ch.is_whitespace() {
                widest = widest.max(measure(&ch.to_string()));
            }
        }
    }
    if !word.is_empty() {
        widest = widest.max(measure(&word));
    }
    // 垫一个「至少一个 em」的底：断行是按整字/整词切的，可用宽再小也不该把盒子
    // 压到放不下一个字（步进器 `- 1 +`、单字标签靠这条撑住，去掉会让 sample
    // 购物车页的双端差异从 1.5% 崩到 10.1%）。
    //
    // 注意这个底**可能超过 max-content**（宋体 `+` 在 36px 下 advance 只有 24），
    // 而 min-content 按定义不可能大于 max-content —— 所以取用方必须夹一次，
    // 见 `measure_text_node` 里的 `min(content_w)`。
    widest.max(font_px)
}

/// 换行判定的亚像素容差（物理像素）。
///
/// 文本盒宽度来自字形度量求和，绘制时逐字累加同样的度量，理论上正好放得下；
/// 但两处的浮点累加顺序不同，末字可能因 1e-3 级误差被判为"超出"而换行。
/// 用半像素容差吸收这个误差 —— 而不是把每个文本盒都加宽几个像素
/// （后者会让所有按内容定宽的元素比浏览器宽一圈）。
pub const WRAP_TOLERANCE_PX: f32 = 0.5;
