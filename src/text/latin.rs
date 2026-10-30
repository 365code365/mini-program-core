//! iOS：西文、数字、半角标点直接用系统字体 SF（宿主设 `MINI_LATIN_FONT` / `MINI_LATIN_BOLD_FONT`，
//! iOS SDK 的 MiniFonts.swift 在 `/System/Library/Fonts/Core/SFUI.ttf` 读得到时设）。
//!
//! 微信 iOS 的页面字体栈第一个是 `-apple-system`，西文就是 SF；汉字才回退到苹方。
//! 苹方是私有的 hvgl 轮廓读不了，只能随包带思源黑体顶替；SF 却是普通的 glyf 可变字体，fontdue 读得了，
//! 而且它就在用户手机上，不用（也不许）随包分发。用真 SF 之后西文的字形、字宽与参考图一致，
//! 合成字体里那份 Inter 只在读不到 SF 时兜底。
//!
//! 字宽另按 SF 的 `trak` 表加字距（CoreText 对系统字体默认就加）：17px 是 −52/2048 em，
//! 一行 40 个字母差出 17px × 0.025 × 40 ≈ 17px。表从 iOS 27 模拟器的 SFUI.ttf 读出（trak.horizData[0.0]）。
use super::*;

/// 西文字面的来源（字体规格见 `text_system::font_spec`：`路径@字重@光学尺寸`）
pub(super) struct Latin {
    regular: String,
    bold: Option<String>,
    faces: [OnceLock<Option<Font>>; 2],
}

impl Latin {
    /// 只在宿主同时指定了主字体（iOS 的合成字体）时生效：桌面、Android 不设这两个变量，行为不变
    pub(super) fn from_env() -> Option<Self> {
        std::env::var_os("MINI_MAIN_FONT")?;
        let regular = std::env::var("MINI_LATIN_FONT").ok()?;
        Some(Self { regular, bold: std::env::var("MINI_LATIN_BOLD_FONT").ok(), faces: Default::default() })
    }
}

/// SF 的默认字距（em），按 CSS 字号查表、表内线性插值；`trak` 的单位是 2048 分之一 em
fn sf_track_em(css: f32) -> f32 {
    const TRAK: [(f32, f32); 21] = [
        (6.0, 82.0), (9.0, 38.0), (10.0, 24.0), (11.0, 12.0), (12.0, 0.0), (13.0, -12.0), (14.0, -22.0),
        (15.0, -32.0), (16.0, -40.0), (17.0, -52.0), (20.0, -46.0), (22.0, -24.0), (24.0, 6.0), (28.0, 28.0),
        (32.0, 26.0), (36.0, 21.0), (50.0, 14.0), (64.0, 7.0), (80.0, 0.0), (100.0, 0.0), (138.0, 0.0),
    ];
    let t = if css <= TRAK[0].0 {
        TRAK[0].1
    } else {
        TRAK.windows(2)
            .find(|w| css <= w[1].0)
            .map(|w| w[0].1 + (w[1].1 - w[0].1) * (css - w[0].0) / (w[1].0 - w[0].0))
            .unwrap_or(0.0)
    };
    t / 2048.0
}

impl TextRenderer {
    /// 这个字该不该用 SF 画：设了西文字体、不是汉字、SF 里有这个字形
    pub(super) fn latin_face(&self, ch: char, bold: bool) -> Option<&Font> {
        let latin = self.latin.as_ref()?;
        if is_cjk_char(ch) {
            return None;
        }
        let i = usize::from(bold && latin.bold.is_some());
        let face = latin.faces[i]
            .get_or_init(|| Self::face_from_file(if i == 1 { latin.bold.as_deref()? } else { &latin.regular }))
            .as_ref()?;
        (face.lookup_glyph_index(ch) != 0).then_some(face)
    }

    /// 字形的前进宽度：iOS 上汉字按苹方 UI 的 em 与 trak 收窄（text/ios_box.rs），SF 画的字加 SF 的字距，
    /// 其余原样。绘制与度量都走这里，二者逐位一致
    #[inline]
    pub(super) fn advance_of(&self, ch: char, size: f32, bold: bool, raw: f32) -> f32 {
        if is_cjk_char(ch) {
            return ios_box::ios_cjk_advance_scale(size).map_or(raw, |k| raw * k);
        }
        if self.latin.is_some() && self.latin_face(ch, bold).is_some() {
            if let Some(css) = ios_box::ios_css_size(size) {
                return raw + size * sf_track_em(css);
            }
        }
        raw
    }
}
