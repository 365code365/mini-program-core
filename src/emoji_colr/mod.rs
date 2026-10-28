//! 彩色 Emoji：`COLR`（v0 图层 / v1 渐变 + 变换 + 合成）矢量字形。
//!
//! Android 12 起系统的 `NotoColorEmoji.ttf` 是 COLRv1（2.8MB，矢量），不再是 CBDT 位图；
//! `emoji.rs` 只认 Apple 的 `sbix` 位图，所以 Android 上所有 emoji 都是豆腐块（小游戏 farm001 的 💰📅💧）。
//! 这里自己画：ttf-parser 负责解析 / 遍历绘制指令，`paint.rs` 负责光栅化与合成（不引第三方 2D 图形库）。
//!
//! 字体：`MINI_EMOJI_FONT=<路径>` 优先（测试、换字体排查用），否则按系统路径找。整个文件读进内存一次
//! （Face 借用它，进程内只开一份）。输出与 `sbix` 那条路同一个 [`EmojiBitmap`]，绘制路径不用改。
//! 按 (字形号, 字号×4) 缓存；画一个 32px 的 emoji 是几十个图层各扫一遍，桌面上约 1ms 量级。
mod paint;
pub(crate) mod raster;

use crate::emoji::EmojiBitmap;
use raster::Affine;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use ttf_parser::{Face, GlyphId, RgbaColor};

const PATHS: &[&str] = &["/system/fonts/NotoColorEmoji.ttf"];

/// 位图四周留的像素：抗锯齿边缘、略出 em 框的笔画别被裁掉
const PAD: f32 = 1.0;

pub struct ColrFont {
    face: Face<'static>,
    upem: f32,
    ascent: f32,
    descent: f32,
    cache: Mutex<HashMap<(u16, u32), Option<Arc<EmojiBitmap>>>>,
}

static SHARED: OnceLock<Option<ColrFont>> = OnceLock::new();

pub fn shared() -> Option<&'static ColrFont> {
    SHARED.get_or_init(load).as_ref()
}

fn load() -> Option<ColrFont> {
    let env = std::env::var("MINI_EMOJI_FONT").ok();
    for path in env.iter().map(String::as_str).chain(PATHS.iter().copied()) {
        let Ok(bytes) = std::fs::read(path) else { continue };
        match ColrFont::from_bytes(bytes) {
            Some(f) => {
                println!("✅ Color emoji（COLR）：{path}");
                return Some(f);
            }
            None => eprintln!("⚠  {path} 不是可用的 COLR 字体"),
        }
    }
    None
}

impl ColrFont {
    /// 解析一份字体；没有 COLR 表返回 None。字节泄漏成 'static：进程里只开一次
    pub fn from_bytes(bytes: Vec<u8>) -> Option<ColrFont> {
        let data: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let face = Face::parse(data, 0).ok()?;
        face.tables().colr?;
        let upem = face.units_per_em() as f32;
        let (ascent, descent) = (face.ascender() as f32, face.descender() as f32);
        Some(ColrFont { face, upem, ascent, descent, cache: Mutex::new(HashMap::new()) })
    }

    pub fn glyph(&self, ch: char) -> Option<u16> {
        let g = self.face.glyph_index(ch)?;
        (g.0 != 0 && self.face.is_color_glyph(g)).then_some(g.0)
    }

    pub fn advance(&self, gid: u16, size: f32) -> f32 {
        self.face.glyph_hor_advance(GlyphId(gid)).unwrap_or(self.upem as u16) as f32 * size / self.upem
    }

    pub fn bitmap(&self, gid: u16, size: f32) -> Option<Arc<EmojiBitmap>> {
        let key = (gid, (size * 4.0).round() as u32);
        if let Some(hit) = self.cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
            return hit;
        }
        let built = self.render(gid, size).map(Arc::new);
        if let Ok(mut c) = self.cache.lock() {
            c.insert(key, built.clone());
        }
        built
    }

    /// 按字号画一枚：位图横向盖住 [0, 前进宽度]、纵向盖住 [descender, ascender]，四周各留 PAD
    pub fn render(&self, gid: u16, size: f32) -> Option<EmojiBitmap> {
        if !(size > 0.0) || size > 1024.0 {
            return None;
        }
        let s = size / self.upem;
        let advance = self.advance(gid, size);
        let w = (advance + 2.0 * PAD).ceil().max(1.0) as usize;
        let h = ((self.ascent - self.descent) * s + 2.0 * PAD).ceil().max(1.0) as usize;
        // 字体单位（y 向上）→ 位图像素（y 向下）
        let base = Affine { a: s, b: 0.0, c: 0.0, d: -s, e: PAD, f: self.ascent * s + PAD };
        let mut p = paint::ColrPainter::new(&self.face, w, h, base);
        // 前景色（palette 0xFFFF 的图层）：emoji 里极少用到，取黑色
        self.face.paint_color_glyph(GlyphId(gid), 0, RgbaColor::new(0, 0, 0, 255), &mut p)?;
        let rgba = p.finish();
        Some(EmojiBitmap {
            width: w as u32,
            height: h as u32,
            rgba,
            draw_w: w as f32,
            draw_h: h as f32,
            left: -PAD,
            bottom: self.descent * s - PAD,
            advance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ColrFont {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fonts/NotoColorEmoji-subset.ttf");
        ColrFont::from_bytes(std::fs::read(p).unwrap()).expect("夹具是 COLRv1 字体")
    }

    /// 💰 是带渐变的 COLRv1 字形：画出来要有足够多的不透明像素、而且不是一种颜色（渐变真的求值了）
    #[test]
    fn 画出彩色而且有渐变() {
        let f = fixture();
        let g = f.glyph('💰').expect("💰 在夹具里");
        let bm = f.render(g, 64.0).unwrap();
        let opaque: Vec<&[u8]> = bm.rgba.chunks(4).filter(|p| p[3] > 200).collect();
        assert!(opaque.len() > (bm.width * bm.height) as usize / 4, "不透明像素太少：{} / {}", opaque.len(), bm.width * bm.height);
        let mut colors: Vec<[u8; 3]> = opaque.iter().map(|p| [p[0] / 16, p[1] / 16, p[2] / 16]).collect();
        colors.sort();
        colors.dedup();
        assert!(colors.len() > 20, "颜色太单一（{} 种），渐变没生效", colors.len());
    }

    /// 字母不是彩色字形：交回常规字体链；不认识的码点也一样
    #[test]
    fn 非彩色字形返回_none() {
        let f = fixture();
        assert_eq!(f.glyph('A'), None);
        assert_eq!(f.glyph('中'), None);
    }

    /// 同一个字号画两次逐字节一样（缓存之外的确定性，基线比对靠它）
    #[test]
    fn 两次绘制逐字节相同() {
        let f = fixture();
        let g = f.glyph('🌈').unwrap();
        assert_eq!(f.render(g, 33.0).unwrap().rgba, f.render(g, 33.0).unwrap().rgba);
    }
}
