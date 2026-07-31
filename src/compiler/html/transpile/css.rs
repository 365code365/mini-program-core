//! WXSS → CSS：rpx 换算、标签选择器改写、基础样式表
//!
//! `transpile_new` 的一片。**纯搬迁**：从 805 行按职责切开，一行逻辑没改。
use super::emit::escape_text;

/// 把字符串中的 `<number>rpx` 换算为 `px`（1rpx = 0.5px，对应 375 逻辑宽），其余不变。
pub fn convert_rpx(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < n {
        // 尝试匹配 数字 + "rpx"
        let start = i;
        let mut j = i;
        if j < n && (chars[j] == '-' || chars[j] == '+') { j += 1; }
        let mut has_digit = false;
        while j < n && (chars[j].is_ascii_digit() || chars[j] == '.') {
            if chars[j].is_ascii_digit() { has_digit = true; }
            j += 1;
        }
        if has_digit && j + 3 <= n && chars[j] == 'r' && chars[j + 1] == 'p' && chars[j + 2] == 'x' {
            let num: String = chars[start..j].iter().collect();
            if let Ok(v) = num.parse::<f64>() {
                // 去掉多余小数
                let px = v * 0.5;
                if (px.fract()).abs() < 1e-6 {
                    out.push_str(&format!("{}px", px as i64));
                } else {
                    out.push_str(&format!("{:.3}px", px));
                }
                i = j + 3;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// 小程序内置元素标签名（这些在 HTML 里被映射成 div/span/img 等，需把选择器改写为 .wx-<tag>）
const WX_TAGS: &[&str] = &[
    "view", "scroll-view", "swiper", "swiper-item", "movable-view", "movable-area",
    "cover-view", "cover-image", "text", "rich-text", "image", "icon", "progress",
    "button", "checkbox", "checkbox-group", "radio", "radio-group", "switch", "slider",
    "input", "textarea", "picker", "picker-view", "picker-view-column", "form", "label",
    "navigator", "video", "audio", "camera", "live-player", "live-pusher", "canvas",
    "map", "web-view", "ad", "block",
];

/// WXSS → CSS：
/// ① rpx → px；② 元素类型选择器改写为 `.wx-<tag>`（因为 HTML 里 view→div、text→span 等，
///    否则 `.foo text{...}` 这类后代标签选择器全部失效，样式大面积丢失）。
pub fn wxss_to_css(src: &str) -> String {
    rewrite_tag_selectors(&convert_rpx(src))
}

/// 判断某标识符是否为需要改写的小程序标签
pub(super) fn is_wx_tag(ident: &str) -> bool {
    WX_TAGS.contains(&ident)
}

/// 改写单个选择器串里的元素类型选择器：`.a text` → `.a .wx-text`，`view>text` → `.wx-view>.wx-text`
fn rewrite_selector_tokens(sel: &str) -> String {
    let chars: Vec<char> = sel.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(sel.len() + 8);
    let mut i = 0;
    let mut attr_depth = 0i32; // 处于 [...] 内不改写
    while i < n {
        let c = chars[i];
        if c == '[' { attr_depth += 1; out.push(c); i += 1; continue; }
        if c == ']' { if attr_depth > 0 { attr_depth -= 1; } out.push(c); i += 1; continue; }
        if attr_depth > 0 { out.push(c); i += 1; continue; }

        if c.is_ascii_alphabetic() {
            // 上一个有效字符（决定是否处于「类型选择器」位置）
            let prev = out.chars().last();
            let type_pos = match prev {
                None => true,
                Some(p) => matches!(p, ' ' | '\t' | '\n' | '\r' | '>' | '+' | '~' | ',' | '('),
            };
            let start = i;
            while i < n && (chars[i].is_ascii_alphanumeric() || chars[i] == '-') { i += 1; }
            let ident: String = chars[start..i].iter().collect();
            if type_pos && ident == "page" {
                // WXSS 根选择器 page → HTML 根容器 #app
                out.push_str("#app");
            } else if type_pos && is_wx_tag(&ident) {
                out.push_str(".wx-");
                out.push_str(&ident);
            } else {
                out.push_str(&ident);
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// 扫描整段 CSS，只在「选择器位置」改写标签选择器；声明块、@media 条件、@keyframes 内容保持原样。
fn rewrite_tag_selectors(css: &str) -> String {
    let mut out = String::with_capacity(css.len() + 64);
    let mut sel = String::new();
    let mut in_block = false;
    let mut depth = 0i32;
    for c in css.chars() {
        if in_block {
            out.push(c);
            if c == '{' { depth += 1; }
            else if c == '}' { depth -= 1; if depth == 0 { in_block = false; } }
            continue;
        }
        // 选择器 / at-rule 前导 区域
        match c {
            '{' => {
                let trimmed = sel.trim_start();
                let low = trimmed.to_ascii_lowercase();
                if low.starts_with("@media") || low.starts_with("@supports") || low.starts_with("@container") {
                    // 分组 at-rule：条件原样输出，内部仍是规则（继续在选择器模式）
                    out.push_str(&sel);
                    out.push('{');
                } else if trimmed.starts_with('@') {
                    // @keyframes / @font-face / @page 等：内容原样，进入块模式
                    out.push_str(&sel);
                    out.push('{');
                    in_block = true;
                    depth = 1;
                } else {
                    out.push_str(&rewrite_selector_tokens(&sel));
                    out.push('{');
                    in_block = true;
                    depth = 1;
                }
                sel.clear();
            }
            '}' => {
                // 关闭分组 at-rule（如 @media）
                out.push_str(&sel);
                sel.clear();
                out.push('}');
            }
            ';' => {
                // 顶层 @import / @charset 等以分号结束的语句
                out.push_str(&sel);
                sel.clear();
                out.push(';');
            }
            _ => sel.push(c),
        }
    }
    out.push_str(&sel);
    out
}

/// 基础样式：CSS reset + 常见默认（贴近小程序默认盒模型）+ 内置 icon 图标。
pub fn base_css() -> String {
    format!("{}{}", BASE_CSS, icon_color_css())
}

/// 内置图标的默认颜色规则：从 `icon_data::icon_default_color` 生成，
/// 免得 CSS 和原生渲染器各写一份颜色表（以前 `circle` 一边 #ccc 一边绿色）。
fn icon_color_css() -> String {
    use crate::renderer::components::icon_data::{icon_default_color, ICON_TYPES};
    let mut s = String::from("\n/* 内置图标默认色（由 icon_data 生成）*/\n");
    for t in ICON_TYPES {
        s.push_str(&format!(".wxicon-{}{{color:#{:06x};}}\n", t, icon_default_color(t)));
    }
    s
}

const BASE_CSS: &str = {
    r#"
*{box-sizing:border-box;margin:0;padding:0;-webkit-tap-highlight-color:transparent;}
@keyframes wxspin{to{transform:rotate(360deg);}}
/* ── 下拉刷新指示器：与原生端同一套外观（64px 区域 + 三个灰点） ── */
.wx-pull-indicator{position:fixed;top:0;left:50%;transform:translateX(-50%);width:375px;height:64px;
  display:flex;align-items:center;justify-content:center;gap:8px;opacity:0;pointer-events:none;z-index:5;}
.wx-pull-indicator>i{width:6px;height:6px;border-radius:50%;background:#888;display:block;}
@keyframes wxpulldot{0%,100%{opacity:.3;}50%{opacity:1;}}
.wx-pull-indicator.is-refreshing>i{animation:wxpulldot .9s ease-in-out infinite;}
.wx-pull-indicator.is-refreshing>i:nth-child(2){animation-delay:.15s;}
.wx-pull-indicator.is-refreshing>i:nth-child(3){animation-delay:.3s;}
#app{transition:transform .25s ease-out;}
/* 还原小程序默认盒模型：view 等默认 flex 纵向排列（与引擎一致，可收缩以适应宽度）*/
.wx-view,.wx-cover-view,.wx-navigator,.wx-checkbox-group,.wx-radio-group,.wx-form,.wx-movable-view,.wx-movable-area{display:flex;flex-direction:column;}
/* scroll-view：块级可滚动容器——子元素保持自身高度、超出则滚动，而非像 flex 那样被压缩 */
.wx-scroll-view{display:block;}
.wx-scroll-view>*{flex-shrink:0;}
/* text：保留数据里的换行（小程序 <text> 中 \n 就是换行），但仍折叠连续空格与缩进 */
.wx-text{display:inline;white-space:pre-line;}
.wx-image{display:block;}
.wx-input,.wx-textarea{border:0;outline:none;background:none;font:inherit;color:inherit;width:100%;}
.wx-navigator{text-decoration:none;color:inherit;}
img{display:block;}
body{font-size:16px;color:#333;font-family:-apple-system,system-ui,"PingFang SC","Hiragino Sans GB",sans-serif;background:#f5f6f8;}

/* ── 交互反馈：带事件的元素显示手型光标 + 按压态 ── */
[data-tap],[data-longpress],[data-longtap],.wx-button,.wx-navigator,.wx-checkbox,.wx-radio,.wx-switch,.wx-slider,.wx-picker{cursor:pointer;}
/* 按压反馈只保留在 button 上：小程序里普通 view 被点击并不会整体变半透明（那是 hover-class 的事）。
   之前给所有 `[data-tap]` 加了 `:active{opacity:.6}`，点遮罩关弹窗时整层会先闪一下再消失，
   而原生端没有这个效果 —— 既是两端不一致，观感上也就是用户说的"闪烁"。 */

/* ── button：还原微信默认按钮盒模型（页面样式只需覆盖颜色等即可保持一致） ── */
.wx-button{position:relative;box-sizing:border-box;display:flex;align-items:center;justify-content:center;min-height:46px;padding:0 14px;font-size:17px;line-height:1.35;text-align:center;border:0;border-radius:5px;background:#f7f7f7;color:#000;cursor:pointer;transition:opacity .12s ease;overflow:hidden;}
.wx-button:active{opacity:.85;}
.wx-button-primary{background:#07c160;color:#fff;}
.wx-button-warn{background:#fa5151;color:#fff;}
.wx-button-plain{background:transparent;border:1px solid currentColor;}
.wx-button-mini{display:inline-flex;width:auto;min-height:30px;padding:0 14px;font-size:13px;border-radius:4px;}
.wx-button-disabled{opacity:.5;pointer-events:none;}

/* ── swiper 轮播：横向 scroll-snap，一屏一页 ── */
.wx-swiper{position:relative;display:flex;flex-direction:row;flex-wrap:nowrap;overflow-x:auto;overflow-y:hidden;height:150px;scroll-snap-type:x mandatory;scroll-behavior:smooth;-webkit-overflow-scrolling:touch;scrollbar-width:none;}
.wx-swiper::-webkit-scrollbar{display:none;width:0;height:0;}
.wx-swiper[data-vertical="true"]{flex-direction:column;overflow-x:hidden;overflow-y:auto;scroll-snap-type:y mandatory;}
.wx-swiper-item{flex:0 0 100%;width:100%;min-width:100%;height:100%;scroll-snap-align:start;display:flex;flex-direction:column;}
.wx-swiper[data-vertical="true"] .wx-swiper-item{flex:0 0 100%;height:100%;}
/* 无 JS（静态首屏/截图）时只显示第一屏，避免各 item 堆叠；JS 接管后加 .wx-swiper-live 恢复多屏滚动 */
.wx-swiper:not(.wx-swiper-live)>.wx-swiper-item:not(:first-child){display:none;}
.wx-swiper-wrap{position:relative;}
.wx-swiper-dots{position:absolute;left:0;right:0;bottom:8px;display:flex;flex-direction:row;justify-content:center;gap:6px;pointer-events:none;}
.wx-swiper-dot{width:7px;height:7px;border-radius:50%;background:rgba(0,0,0,.3);transition:background .2s;}
.wx-swiper-dot.active{background:#fff;box-shadow:0 0 2px rgba(0,0,0,.4);}

/* ── checkbox / radio：自绘微信风格（圆角方框 / 圆形，选中填充微信绿 + 白勾）── */
.wx-checkbox,.wx-radio{-webkit-appearance:none;appearance:none;width:22px;height:22px;margin:0;flex:none;cursor:pointer;box-sizing:border-box;border:1px solid #cfcfcf;background:#fff;transition:background .15s,border-color .15s;vertical-align:middle;}
.wx-checkbox{border-radius:4px;}
.wx-radio{border-radius:50%;}
.wx-checkbox:checked{background:#07c160 url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'%3E%3Cpath d='M3.5 8.5 6.5 11.5 12.5 5' fill='none' stroke='white' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E") center/15px no-repeat;border-color:#07c160;}
.wx-radio:checked{border-color:#07c160;background:#07c160;box-shadow:inset 0 0 0 3px #fff;}
.wx-checkbox:disabled,.wx-radio:disabled{background:#e6e6e6;border-color:#dcdcdc;}

/* ── switch 开关：还原微信(weui)度量——轨道 52x32、1px 边框、滑块直径 30 ──
   关态是「浅灰边框 + 内部白色胶囊」，开态胶囊缩到 0 露出主色，滑块带投影并缓动位移 */
.wx-switch{flex:none;box-sizing:border-box;width:52px;height:32px;border:1px solid #dfdfdf;border-radius:16px;background:#dfdfdf;position:relative;transition:background-color .2s,border-color .2s;cursor:pointer;display:inline-block;vertical-align:middle;}
.wx-switch::before{content:"";position:absolute;top:0;left:0;width:50px;height:30px;border-radius:15px;background:#fdfdfd;transition:transform .35s cubic-bezier(.45,1,.4,1);}
.wx-switch.wx-switch-on{background:#04be02;border-color:#04be02;}
.wx-switch.wx-switch-on::before{transform:scale(0);}
.wx-switch-knob{position:absolute;top:0;left:0;width:30px;height:30px;border-radius:50%;background:#fff;box-shadow:0 1px 3px rgba(0,0,0,.4);transition:transform .35s cubic-bezier(.4,.4,.25,1.35);}
.wx-switch.wx-switch-on .wx-switch-knob{transform:translateX(20px);}
.wx-switch-disabled{opacity:.45;pointer-events:none;}

/* ── slider 滑块（wrapper + 轨道 + 数值） ── */
.wx-slider-wrap{display:flex;flex-direction:row;align-items:center;}
.wx-slider-value{margin-left:12px;font-size:14px;color:#999;min-width:2.2em;text-align:center;}
.wx-slider{-webkit-appearance:none;appearance:none;width:100%;height:4px;border-radius:2px;background:#e5e5e5;accent-color:#09bb07;cursor:pointer;}
.wx-slider::-webkit-slider-thumb{-webkit-appearance:none;appearance:none;width:22px;height:22px;border-radius:50%;background:#fff;box-shadow:0 1px 4px rgba(0,0,0,.3);}

/* ── picker 底部选择面板（点 picker 弹出，与原生窗体的选择器一致）── */
.wxpk-mask{position:fixed;inset:0;background:rgba(0,0,0,0);z-index:9998;transition:background .22s ease;}
.wxpk-mask.show{background:rgba(0,0,0,.5);}
.wxpk-sheet{position:fixed;left:0;right:0;bottom:0;background:#fff;z-index:9999;transform:translateY(100%);transition:transform .22s cubic-bezier(.2,.8,.2,1);}
.wxpk-sheet.show{transform:translateY(0);}
.wxpk-head{display:flex;align-items:center;justify-content:space-between;height:45px;border-bottom:1px solid #e5e5e5;font-size:17px;}
.wxpk-cancel,.wxpk-confirm{padding:0 16px;line-height:45px;cursor:pointer;}
.wxpk-cancel{color:#888;}
.wxpk-confirm{color:#576b95;}
.wxpk-cols{display:flex;height:220px;position:relative;}
/* 选中行的上下参考线，落在正中一格 */
.wxpk-cols::before,.wxpk-cols::after{content:"";position:absolute;left:0;right:0;height:1px;background:#d9d9d9;z-index:2;pointer-events:none;}
.wxpk-cols::before{top:88px;}
.wxpk-cols::after{top:132px;}
.wxpk-col{flex:1;overflow-y:auto;scroll-snap-type:y mandatory;-webkit-overflow-scrolling:touch;scrollbar-width:none;text-align:center;}
.wxpk-col::-webkit-scrollbar{display:none;width:0;height:0;}
/* 上下各留两格空白，让首/末项也能滚到正中 */
.wxpk-col-pad{height:88px;}
.wxpk-item{height:44px;line-height:44px;font-size:17px;color:#000;scroll-snap-align:center;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}

/* ── progress 进度条 ── */
.wx-progress{display:flex;flex-direction:row;align-items:center;}
.wx-progress-outer{flex:1;height:6px;border-radius:3px;background:#ebebeb;overflow:hidden;}
.wx-progress-inner{height:100%;background:#09bb07;border-radius:3px;transition:width .3s;}
.wx-progress-info{margin-left:8px;font-size:12px;color:#666;min-width:2.5em;text-align:right;}

/* ── tabBar 底部导航（固定，居中对齐 375 宽）── */
.wx-tabbar{position:fixed;left:50%;transform:translateX(-50%);bottom:0;width:375px;max-width:100%;height:50px;display:flex;flex-direction:row;border-top:1px solid #ededed;z-index:500;box-shadow:0 -1px 6px rgba(0,0,0,.04);}
.wx-tabbar-item{flex:1;display:flex;flex-direction:column;align-items:center;justify-content:center;text-decoration:none;padding-top:4px;}
.wx-tabbar-item svg{width:24px;height:24px;display:block;}
.wx-tabbar-text{font-size:11px;line-height:1;margin-top:3px;}

/* ── 媒体 ── */
.wx-video{display:block;width:100%;height:225px;background:#000;}
.wx-canvas{display:block;}
.wx-audio{display:block;width:100%;}

/* 内置矢量图标：内联 SVG，图形数据取自 WeUI（见 icon_data.rs），单色 + even-odd 挖洞。
   默认色规则由 icon_color_css() 生成，追加在这段常量后面。*/
.wxicon{display:inline-flex;align-items:center;justify-content:center;width:23px;height:23px;flex:none;font-style:normal;line-height:0;vertical-align:middle;}
.wxicon>svg{width:100%;height:100%;display:block;}
"#
};

/// 生成一个自包含的静态 HTML 文档（用于静态导出）。
pub fn make_html_doc(title: &str, css: &str, body_html: &str, width_px: u32) -> String {
    format!(
        "<!doctype html>\n<html lang=\"zh\"><head><meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{title}</title>\n<style>\n{base}\n#app{{width:{w}px;min-height:100vh;margin:0 auto;background:#f5f6f8;position:relative;overflow:hidden;}}\n{css}\n</style></head>\n\
<body><div id=\"app\">{body}</div></body></html>",
        title = escape_text(title),
        base = base_css(),
        w = width_px,
        css = css,
        body = body_html,
    )
}
