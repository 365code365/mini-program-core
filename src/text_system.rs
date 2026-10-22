//! 宿主系统默认字体的候选路径（按平台），供 [`crate::text::TextRenderer::load_system_font`] 依次尝试。
//!
//! 为什么单独一个文件：Android 上要读 `/system/etc/fonts.xml` 才知道「这台机器显示中文用的是哪个字体」，
//! 各家 ROM 的文件名都不一样（一加/OPPO `SysSans-Hans-Regular.ttf`、小米 `MiSansVF.ttf`、AOSP `NotoSansCJK`），
//! 写死 macOS 路径的结果是 Android 上一律落到内置的 ArialUnicode —— 字形发虚、西文难看就是这么来的。
//!
//! 一加 PLY110（ColorOS）上实测的 `fonts.xml`：
//! - AOSP 的 `NotoSansCJKjp-Regular.otc` 那几个 `zh-Hans` 家族被 ROM 包在 `<!-- -->` 里（机器上也没有这个文件），
//!   不剥注释就会先挑中它；
//! - `sans-serif` 家族是 `SysFont-Regular.ttf`（OPPO Sans EN，只有 1020 个西文字形，没有汉字），
//!   真正的中文 UI 字体是后面 `zh-Hans` 家族的 `SysSans-Hans-Regular.ttf`（OPPO Sans 4.0 SC，可变字体，默认 wght 400）。
//!
//! 所以每个候选都要「真能读出来、真能解析、（需要时）真有汉字」才算数，按顺序往下退。

/// 一个候选：文件路径、TTC 集合里的字面序号、是否必须带汉字（`sans-serif` 可能只有西文）
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub path: String,
    pub index: u32,
    pub need_cjk: bool,
}

impl Candidate {
    fn new(path: &str, index: u32, need_cjk: bool) -> Self {
        Self { path: path.to_string(), index, need_cjk }
    }
}

/// macOS 桌面：与从前写死的顺序逐字一致（画廊 / 快照基线都按这份字体定的，不能动）
const MACOS: &[&str] = &[
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/Library/Fonts/Arial Unicode.ttf",
    "/System/Library/Fonts/STHeiti Light.ttc",
];

/// `fonts.xml` 都失效时的兜底（常见 ROM 的中文字体文件名；NotoSansCJK 集合里简体中文是第 2 个字面）
#[cfg(target_os = "android")]
const ANDROID_STATIC: &[(&str, u32)] = &[
    ("/system/fonts/SysSans-Hans-Regular.ttf", 0),
    ("/system/fonts/MiSansVF.ttf", 0),
    ("/system/fonts/NotoSansCJK-Regular.ttc", 2),
    ("/system/fonts/NotoSansSC-Regular.otf", 0),
    ("/system/fonts/DroidSansFallbackFull.ttf", 0),
    ("/system/fonts/DroidSansFallback.ttf", 0),
];

/// iOS：未在真机上验证过路径（本机没有 Xcode / 模拟器运行时）；不存在就跳过，最后退内置字体
#[cfg(target_os = "ios")]
const IOS: &[&str] = &[
    "/System/Library/Fonts/LanguageSupport/PingFang.ttc",
    "/System/Library/Fonts/Core/PingFang.ttc",
    "/System/Library/Fonts/CoreAddition/PingFang.ttc",
];

/// 本机的主字体候选（按优先级）
pub fn main_font_candidates() -> Vec<Candidate> {
    #[allow(unused_mut)]
    let mut out = Vec::new();
    #[cfg(target_os = "android")]
    {
        if let Ok(xml) = std::fs::read_to_string("/system/etc/fonts.xml") {
            out.extend(parse_fonts_xml(&xml, "/system/fonts"));
        }
        out.extend(ANDROID_STATIC.iter().map(|(p, i)| Candidate::new(p, *i, false)));
    }
    #[cfg(target_os = "ios")]
    out.extend(IOS.iter().map(|p| Candidate::new(p, 0, false)));
    out.extend(MACOS.iter().map(|p| Candidate::new(p, 0, false)));
    out
}

/// 从 Android 的 `fonts.xml` 里取候选：先 `sans-serif` 家族的常规字面（须带汉字才用 ——
/// 小米这类 ROM 的系统 UI 字体本身就含中文），再按出现顺序取每个 `zh-Hans` 家族的常规字面。
///
/// 不引 XML 库：只认 `<family …>…</family>` 与其中的 `<font …>文件名`，够用且不怕 ROM 加私有属性。
/// 注意 `fonts.xml` 里的注释可能夹着示例标签，先剥掉 `<!-- … -->`。
pub fn parse_fonts_xml(xml: &str, dir: &str) -> Vec<Candidate> {
    let xml = strip_comments(xml);
    let mut sans = Vec::new();
    let mut hans = Vec::new();
    let mut rest = xml.as_str();
    while let Some(start) = rest.find("<family") {
        let after = &rest[start..];
        // `<familyset>` 也以 `<family` 开头：不跳过的话它会把第一个家族整个吞掉
        if !after[7..].starts_with(|c: char| c.is_whitespace() || c == '>') {
            rest = &after[7..];
            continue;
        }
        let Some(head_end) = after.find('>') else { break };
        let head = &after[..head_end];
        let end = after.find("</family>").unwrap_or(after.len());
        let body = &after[head_end..end];
        rest = &after[end.min(after.len())..];
        if rest.starts_with("</family>") {
            rest = &rest["</family>".len()..];
        }
        let is_sans = attr(head, "name") == Some("sans-serif");
        let is_hans = attr(head, "lang").is_some_and(|l| l.split(',').any(|x| x.trim() == "zh-Hans"));
        if !is_sans && !is_hans {
            continue;
        }
        let Some((file, index)) = regular_font(body) else { continue };
        let path = format!("{dir}/{file}");
        if is_sans {
            sans.push(Candidate::new(&path, index, true));
        } else {
            hans.push(Candidate::new(&path, index, false));
        }
    }
    let mut out = sans;
    for c in hans {
        if !out.iter().any(|o| o.path == c.path && o.index == c.index) {
            out.push(c);
        }
    }
    out
}

fn strip_comments(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(i) = rest.find("<!--") {
        out.push_str(&rest[..i]);
        match rest[i..].find("-->") {
            Some(j) => rest = &rest[i + j + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// 家族里的常规字面：优先 `weight="400"` 且非 `fallbackFor`（那是给衬线体的），没有就取第一个
fn regular_font(body: &str) -> Option<(String, u32)> {
    let mut first = None;
    let mut rest = body;
    while let Some(i) = rest.find("<font") {
        let after = &rest[i + 5..];
        rest = after;
        // `<font>` 与 `<fontfamily>` 之类区分开：后面必须是空白或 `>`
        if !after.starts_with(|c: char| c.is_whitespace() || c == '>') {
            continue;
        }
        let Some(head_end) = after.find('>') else { break };
        let head = &after[..head_end];
        let text = &after[head_end + 1..];
        let file: String = text.chars().take_while(|c| *c != '<' && !c.is_whitespace()).collect();
        if file.is_empty() {
            continue;
        }
        let index = attr(head, "index").and_then(|v| v.parse().ok()).unwrap_or(0);
        let item = (file, index);
        if attr(head, "weight") == Some("400") && attr(head, "fallbackFor").is_none() && attr(head, "style") != Some("italic") {
            return Some(item);
        }
        first.get_or_insert(item);
    }
    first
}

fn attr<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let i = head.find(&key)? + key.len();
    let j = head[i..].find('"')?;
    Some(&head[i..i + j])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一加 PLY110 的 `fonts.xml` 摘录（结构原样，家族删减；注释里那段对应 ROM 注释掉的 AOSP Noto）
    const COLOROS: &str = r#"<familyset>
    <!--
    <family lang="zh-Hans"><font weight="400" style="normal" index="2">Commented-Out.otc</font></family>
    --> <!--  #else /* OPLUS_FEATURE_FONT_FLIP */-->
    <family name="sans-serif">
        <font weight="100" style="normal">SysFont-Regular.ttf
            <axis tag="wght" stylevalue="100" />
        </font>
        <font weight="400" style="normal">SysFont-Regular.ttf
            <axis tag="wght" stylevalue="400" />
        </font>
    </family>
    <family lang="und-Arab" variant="elegant"><font weight="400" style="normal">NotoNaskhArabic-Regular.ttf</font></family>
    <family lang="zh-Hans">
        <font weight="400" style="normal" index="2">NotoSansCJKjp-Regular.otc</font>
        <font weight="400" style="normal" index="2" fallbackFor="serif">NotoSerifCJKjp-Regular.otc</font>
    </family>
    <family lang="zh-Hant,zh-Bopo"><font weight="400" style="normal" index="3">NotoSansCJKjp-Regular.otc</font></family>
    <family lang="zh-Hans">
        <font weight="100" style="normal"  postScriptName="OPPO_Sans_4.0_SC" >SysSans-Hans-Regular.ttf
            <axis tag="wght" stylevalue="100"/>
        </font>
        <font weight="400" style="normal"  postScriptName="OPPO_Sans_4.0_SC" >SysSans-Hans-Regular.ttf
            <axis tag="wght" stylevalue="400"/>
        </font>
    </family>
</familyset>"#;

    #[test]
    fn coloros_fonts_xml_yields_sans_then_hans_in_order() {
        let c = parse_fonts_xml(COLOROS, "/system/fonts");
        assert_eq!(
            c,
            vec![
                Candidate::new("/system/fonts/SysFont-Regular.ttf", 0, true),
                Candidate::new("/system/fonts/NotoSansCJKjp-Regular.otc", 2, false),
                Candidate::new("/system/fonts/SysSans-Hans-Regular.ttf", 0, false),
            ]
        );
    }

    #[test]
    fn desktop_candidates_keep_macos_order() {
        let c = main_font_candidates();
        let tail: Vec<&str> = c.iter().rev().take(MACOS.len()).rev().map(|c| c.path.as_str()).collect();
        assert_eq!(tail, MACOS);
    }
}
