//! 字体占了多少内存（`memory_report` 里那一段；mem_report 示例与宿主日志用）
use super::TextRenderer;

/// 一个渲染器（主字面 + 已经用到的粗体 / 符号 / emoji 字面）的堆占用
#[derive(Debug, Default, Clone, Copy)]
pub struct FontMem {
    /// 解析了几个字面
    pub faces: usize,
    /// 展开了几何的字形数
    pub glyphs: usize,
    /// 字形几何（字节）
    pub geometry: usize,
    /// 字符 → 字形表与槽位（字节）
    pub tables: usize,
    /// 字形位图缓存：条数与字节
    pub bitmaps: (usize, usize),
}

impl FontMem {
    pub fn total(&self) -> usize {
        self.geometry + self.tables + self.bitmaps.1
    }
}

impl TextRenderer {
    /// 字形位图缓存清空（内存告警时；下一帧要画的字重新光栅化）
    pub fn clear_glyph_bitmaps(&self) {
        if let Ok(mut c) = self.cache.lock() {
            *c = Default::default();
        }
    }

    pub fn font_memory(&self) -> FontMem {
        let mut m = FontMem::default();
        let faces = [Some(&self.main_font), self.bold_font.get().and_then(|f| f.as_ref()), self.symbol_font.get().and_then(|f| f.as_ref()), self.emoji_font.as_ref()];
        for f in faces.into_iter().flatten() {
            let (g, t) = f.heap_bytes();
            m.faces += 1;
            m.glyphs += f.loaded_glyph_count();
            m.geometry += g;
            m.tables += t;
        }
        if let Ok(c) = self.cache.lock() {
            m.bitmaps = (c.len(), c.values().map(|(_, b)| b.len() + 64).sum());
        }
        m
    }
}

/// 进程共享的系统字体那一份（一行，接 `memory_report`）
pub fn report() -> String {
    let Some(r) = super::shared_fonts() else { return "字体：没加载".into() };
    let m = r.font_memory();
    let mb = |b: usize| b as f32 / 1048576.0;
    format!(
        "字体 {} 个字面：展开字形 {} 个 {:.1}MB、表 {:.1}MB、位图缓存 {} 条 {:.1}MB",
        m.faces, m.glyphs, mb(m.geometry), mb(m.tables), m.bitmaps.0, mb(m.bitmaps.1)
    )
}
