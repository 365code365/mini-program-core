//! iOS WebKit 的行盒与基线（宿主设 `MINI_LINE_BOX=ios@<屏幕倍率>`，iOS SDK 的 MiniFonts.swift 设）。
//!
//! 引擎默认的行高系数（text.rs 的 CJK / LATIN_LINE_HEIGHT_FACTOR）是在 macOS 的 Chrome 上量的。
//! iOS 的 WebKit（微信 iOS 的 H5 / WebView 页面也是它）不一样：行盒由字体栈的第一个字体
//! `-apple-system`（SF）决定，汉字行、西文行都一样。上 / 下伸 = ceil(字号 × 1950/2048) / ceil(字号 × 494/2048)，
//! 单位 CSS 像素（SF UI 的 hhea 各自向上取整）。iPhone 17 Pro 模拟器上 10~20px 逐个字号量过，十个全中
//! （target/ios_ref 的探针页 lh.html）。按 Chrome 的 1.375 算，16px 汉字行是 22 而 iOS 是 20，
//! 一屏几十行累起来整页往下错开十几个点。
//!
//! 基线同样按 SF 的上 / 下伸放：基线 = 行顶 + (行高 − (上伸 + 下伸)) / 2 + 上伸（半行距可以是负的）。
//! 引擎原来的多行文字把首行基线放在「行顶 + 字号」，不管行高：news 详情页 24px 标题配 28px 行高时
//! 字比 iOS 低 1.5px，17px 正文配 26px 行高时高 2px（WebKit 参考图逐行量出 +4 / −7 物理像素，@3x）。
//! 不是 iOS 时都返回 None，调用方走原来的口径（桌面基线不动）。
use std::sync::OnceLock;

fn ios_dpr() -> Option<f32> {
    static DPR: OnceLock<Option<f32>> = OnceLock::new();
    *DPR.get_or_init(|| {
        let v = std::env::var("MINI_LINE_BOX").ok()?;
        let s = v.strip_prefix("ios")?;
        Some(s.strip_prefix('@').and_then(|d| d.parse::<f32>().ok()).filter(|d| *d > 0.0).unwrap_or(1.0))
    })
}

/// 物理字号换成 CSS 字号；None = 不是 iOS
pub(super) fn ios_css_size(physical_size: f32) -> Option<f32> {
    Some(physical_size / ios_dpr()?)
}

/// SF 的 (上伸, 下伸)，物理像素（按 CSS 像素取整后再乘倍率）
fn ios_ascent_descent(physical_size: f32) -> Option<(f32, f32)> {
    let dpr = ios_dpr()?;
    let css = physical_size / dpr;
    Some(((css * 1950.0 / 2048.0).ceil() * dpr, (css * 494.0 / 2048.0).ceil() * dpr))
}

/// `line-height: normal` 的行高，物理像素；None = 不是 iOS
pub(super) fn ios_line_box(physical_size: f32) -> Option<f32> {
    ios_ascent_descent(physical_size).map(|(a, d)| a + d)
}

/// 一行里基线距行顶的距离（物理像素）；None = 不是 iOS
pub fn ios_baseline(physical_size: f32, line_height: f32) -> Option<f32> {
    ios_ascent_descent(physical_size).map(|(a, d)| (line_height - (a + d)) / 2.0 + a)
}

/// 汉字（含全角标点）前进宽度的缩放，物理字号进；None = 不是 iOS。
///
/// iOS 的 `-apple-system` 汉字回退到 `.PingFang UI Text`：unitsPerEm 是 **1028**，汉字前进宽 1000，
/// 再按字号查 `trak` 表加字距（单位同 em）。所以 17px 正文一个字宽 17 × 1019/1028 = 16.85px，不是 17：
/// 20 个字一行比引擎窄 3.4px（WebKit 参考图 @3x 逐字量出的字距 50.53 物理像素 / 字，按这张表算 50.56）。
/// 表从 iOS 26 模拟器的 PingFangUI.ttc 读出（fontTools 的 trak.horizData[1.0]），表内线性插值、表外取端点，
/// 与 CoreText 一致
pub fn ios_cjk_advance_scale(physical_size: f32) -> Option<f32> {
    const TRAK: [(f32, f32); 14] = [
        (10.0, 22.0), (11.0, 22.0), (12.0, 21.0), (13.0, 20.0), (14.0, 20.0), (15.0, 20.0), (16.0, 20.0),
        (17.0, 19.0), (20.0, 19.0), (22.0, 13.0), (28.0, 13.0), (32.0, 9.0), (36.0, 8.0), (50.0, 4.0),
    ];
    let css = physical_size / ios_dpr()?;
    let track = if css <= TRAK[0].0 {
        TRAK[0].1
    } else {
        TRAK.windows(2)
            .find(|w| css <= w[1].0)
            .map(|w| w[0].1 + (w[1].1 - w[0].1) * (css - w[0].0) / (w[1].0 - w[0].0))
            .unwrap_or(TRAK[TRAK.len() - 1].1)
    };
    Some((1000.0 + track) / 1028.0)
}

/// 彩色 emoji 在 iOS 上的画法，物理字号进：(画多大的字号, 前进宽度额外加多少, 往下挪多少)，都是物理像素；
/// None = 不是 iOS。
///
/// iOS 的 CoreText 把小字号的 Apple Color Emoji 放大画（字号 ≤16px 时放大到 1.27 倍，16→24px 之间
/// 线性收回到 1 倍），前进宽度再多给 0.125em（左右各一半），放大的部分约 0.3 落到基线以下。
/// 不按这个画，news 搜索页「🔥 热搜榜」的火苗只有参考图的 0.79 倍，后面的字往左错 5px。
/// 数字是 iOS 27 模拟器上 WKWebView 逐字号（10~30px）量的 😀 墨迹与前进宽度拟合的（target/ios_ref 的 emoji.html）
pub fn ios_emoji(physical_size: f32) -> Option<(f32, f32, f32)> {
    let dpr = ios_dpr()?;
    let css = physical_size / dpr;
    let drawn = if css <= 16.0 {
        css * 1.273
    } else if css < 24.0 {
        16.0 * 1.273 + (css - 16.0) * (24.0 - 16.0 * 1.273) / 8.0
    } else {
        css
    };
    Some((drawn * dpr, 0.125 * physical_size, 0.3 * (drawn - css) * dpr))
}
