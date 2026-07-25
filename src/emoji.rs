//! 彩色 Emoji 位图字形（Apple `sbix`）解码。
//!
//! 为什么需要单独一套：Apple Color Emoji 是**位图**字体（`sbix` 表里存的是 PNG），
//! 而 `fontdue` 只能光栅化轮廓字形（`glyf`/`CFF`）。把它塞给 fontdue 的结果是
//! ——花 1.5 秒解析完，却什么也画不出来，emoji 全渲染成豆腐块。
//!
//! 这里直接按 OpenType/`sbix` 规范读需要的几张表（`cmap` / `hhea` / `hmtx` /
//! `maxp` / `head` / `sbix`），用 `seek` 精确取字形 PNG，避免把 183MB 字体整个
//! 读进内存；解码后的位图按 (字形号, 字号) 缓存，绘制路径只做一次 blit。
//!
//! 覆盖范围：单码点 emoji。ZWJ 组合（如 👨‍👩‍👧）、旗帜、keycap 序列不做字形合成，
//! 会退回逐码点绘制。

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex, OnceLock};

/// 候选彩色 emoji 字体（按优先级）
const EMOJI_FONT_PATHS: &[&str] = &[
    "/System/Library/Fonts/Apple Color Emoji.ttc",
    "/Library/Fonts/Apple Color Emoji.ttc",
    "/System/Library/Fonts/AppleColorEmoji.ttf",
];

/// 一枚按目标字号解码好的彩色字形位图
pub struct EmojiBitmap {
    /// 位图像素尺寸
    pub width: u32,
    pub height: u32,
    /// RGBA8 像素
    pub rgba: Vec<u8>,
    /// 绘制宽高（px，已按目标字号换算）
    pub draw_w: f32,
    pub draw_h: f32,
    /// 位图左边相对笔位的偏移（px）
    pub left: f32,
    /// 位图底边相对基线的偏移（px，正值表示位图底边在基线之上）
    pub bottom: f32,
    /// 字形前进宽度（px）
    pub advance: f32,
}

pub struct ColorEmojiFont {
    file: Mutex<File>,
    /// 码点 -> 字形号
    cmap: HashMap<u32, u16>,
    /// sbix strike：(ppem, 文件内绝对偏移)，按 ppem 升序
    strikes: Vec<(u16, u64)>,
    num_glyphs: u16,
    units_per_em: f32,
    /// hmtx 前进宽度（font units），最后一项对尾部字形复用
    advances: Vec<u16>,
    cache: Mutex<HashMap<(u16, u32), Option<Arc<EmojiBitmap>>>>,
}

static SHARED: OnceLock<Option<ColorEmojiFont>> = OnceLock::new();

/// 进程内共享的彩色 emoji 字体（首次调用时惰性打开并解析表目录）。
pub fn shared() -> Option<&'static ColorEmojiFont> {
    SHARED.get_or_init(ColorEmojiFont::load_system).as_ref()
}

/// 该码点是否有彩色位图字形可用。
pub fn has_color_glyph(ch: char) -> bool {
    shared().map(|f| f.glyph_index(ch).is_some()).unwrap_or(false)
}

/// 取指定字号下的彩色字形位图（含绘制所需的偏移与前进宽度）。
pub fn glyph_bitmap(ch: char, size: f32) -> Option<Arc<EmojiBitmap>> {
    let font = shared()?;
    let gid = font.glyph_index(ch)?;
    font.bitmap(gid, size)
}

/// 取指定字号下的前进宽度（无位图时返回 None）。
pub fn advance(ch: char, size: f32) -> Option<f32> {
    let font = shared()?;
    let gid = font.glyph_index(ch)?;
    Some(font.advance_px(gid, size))
}

/// 零宽控制码点：变体选择符 / ZWJ / 肤色修饰前缀等，不参与绘制与度量。
pub fn is_zero_width(ch: char) -> bool {
    let c = ch as u32;
    matches!(c, 0xFE00..=0xFE0F | 0x200D | 0x200B..=0x200C | 0xE0020..=0xE007F)
}

impl ColorEmojiFont {
    fn load_system() -> Option<ColorEmojiFont> {
        for path in EMOJI_FONT_PATHS {
            if !std::path::Path::new(path).exists() {
                continue;
            }
            match Self::open(path) {
                Ok(font) => {
                    println!(
                        "✅ Color emoji: {} ({} strikes, {} glyphs)",
                        path,
                        font.strikes.len(),
                        font.num_glyphs
                    );
                    return Some(font);
                }
                Err(err) => eprintln!("⚠️  彩色 emoji 字体不可用 {}: {}", path, err),
            }
        }
        None
    }

    fn open(path: &str) -> Result<ColorEmojiFont, String> {
        let mut file = File::open(path).map_err(|e| e.to_string())?;

        // TTC 集合：取第一个字体的表目录偏移
        let head = read_at(&mut file, 0, 12)?;
        let font_offset = if &head[0..4] == b"ttcf" {
            let num_fonts = be_u32(&head, 8);
            if num_fonts == 0 {
                return Err("ttcf 内没有字体".into());
            }
            let offsets = read_at(&mut file, 12, 4)?;
            be_u32(&offsets, 0) as u64
        } else {
            0
        };

        // 表目录
        let dir_head = read_at(&mut file, font_offset, 12)?;
        let num_tables = be_u16(&dir_head, 4) as usize;
        let dir = read_at(&mut file, font_offset + 12, num_tables * 16)?;
        let mut tables: HashMap<[u8; 4], (u64, u32)> = HashMap::new();
        for i in 0..num_tables {
            let rec = &dir[i * 16..i * 16 + 16];
            let tag = [rec[0], rec[1], rec[2], rec[3]];
            tables.insert(tag, (be_u32(rec, 8) as u64, be_u32(rec, 12)));
        }
        let table = |tag: &[u8; 4]| tables.get(tag).copied();

        let (sbix_off, _) = table(b"sbix").ok_or("缺少 sbix 表（不是位图彩色字体）")?;
        let (head_off, _) = table(b"head").ok_or("缺少 head 表")?;
        let (maxp_off, _) = table(b"maxp").ok_or("缺少 maxp 表")?;
        let (cmap_off, cmap_len) = table(b"cmap").ok_or("缺少 cmap 表")?;

        let head_buf = read_at(&mut file, head_off, 54)?;
        let units_per_em = be_u16(&head_buf, 18) as f32;
        let units_per_em = if units_per_em > 0.0 { units_per_em } else { 1000.0 };

        let maxp_buf = read_at(&mut file, maxp_off, 6)?;
        let num_glyphs = be_u16(&maxp_buf, 4);

        // hmtx 前进宽度
        let mut advances = Vec::new();
        if let (Some((hhea_off, _)), Some((hmtx_off, hmtx_len))) = (table(b"hhea"), table(b"hmtx")) {
            let hhea = read_at(&mut file, hhea_off, 36)?;
            let num_h_metrics = be_u16(&hhea, 34) as usize;
            let want = (num_h_metrics * 4).min(hmtx_len as usize);
            if want >= 4 {
                let hmtx = read_at(&mut file, hmtx_off, want)?;
                advances = (0..want / 4).map(|i| be_u16(&hmtx, i * 4)).collect();
            }
        }
        if advances.is_empty() {
            advances.push(units_per_em as u16);
        }

        // cmap：码点 -> 字形号
        let cmap_buf = read_at(&mut file, cmap_off, cmap_len.min(1 << 22) as usize)?;
        let cmap = parse_cmap(&cmap_buf).ok_or("cmap 无可用子表")?;

        // sbix strikes
        let sbix_head = read_at(&mut file, sbix_off, 8)?;
        let num_strikes = be_u32(&sbix_head, 4) as usize;
        if num_strikes == 0 {
            return Err("sbix 无 strike".into());
        }
        let offsets = read_at(&mut file, sbix_off + 8, num_strikes * 4)?;
        let mut strikes = Vec::with_capacity(num_strikes);
        for i in 0..num_strikes {
            let strike_abs = sbix_off + be_u32(&offsets, i * 4) as u64;
            let strike_head = read_at(&mut file, strike_abs, 4)?;
            strikes.push((be_u16(&strike_head, 0), strike_abs));
        }
        strikes.sort_by_key(|(ppem, _)| *ppem);

        Ok(ColorEmojiFont {
            file: Mutex::new(file),
            cmap,
            strikes,
            num_glyphs,
            units_per_em,
            advances,
            cache: Mutex::new(HashMap::new()),
        })
    }

    fn glyph_index(&self, ch: char) -> Option<u16> {
        let gid = *self.cmap.get(&(ch as u32))?;
        if gid == 0 || gid >= self.num_glyphs {
            return None;
        }
        Some(gid)
    }

    fn advance_px(&self, gid: u16, size: f32) -> f32 {
        let idx = (gid as usize).min(self.advances.len() - 1);
        self.advances[idx] as f32 * size / self.units_per_em
    }

    /// 选择不小于目标像素尺寸的最小 strike（都更小则取最大的那个）。
    fn pick_strike(&self, target_px: f32) -> (u16, u64) {
        for &(ppem, off) in &self.strikes {
            if ppem as f32 >= target_px {
                return (ppem, off);
            }
        }
        *self.strikes.last().unwrap()
    }

    fn bitmap(&self, gid: u16, size: f32) -> Option<Arc<EmojiBitmap>> {
        let key = (gid, (size * 4.0).round() as u32);
        if let Some(hit) = self.cache.lock().ok().and_then(|c| c.get(&key).cloned()) {
            return hit;
        }
        let built = self.build_bitmap(gid, size).map(Arc::new);
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(key, built.clone());
        }
        built
    }

    fn build_bitmap(&self, gid: u16, size: f32) -> Option<EmojiBitmap> {
        let (ppem, strike) = self.pick_strike(size);
        let (origin_x, origin_y, png) = self.glyph_png(strike, gid, 0)?;

        let decoded = image::load_from_memory(&png).ok()?;
        let rgba = decoded.to_rgba8();
        let (src_w, src_h) = (rgba.width(), rgba.height());
        if src_w == 0 || src_h == 0 {
            return None;
        }

        // strike 的位图是按 ppem 设计的：1 位图像素 = size/ppem 个目标像素
        let px_per_bitmap_px = size / ppem as f32;
        let draw_w = src_w as f32 * px_per_bitmap_px;
        let draw_h = src_h as f32 * px_per_bitmap_px;

        // 预缩放到目标尺寸，绘制路径只做整数 blit（比每帧重采样便宜得多）
        let dst_w = draw_w.round().max(1.0) as u32;
        let dst_h = draw_h.round().max(1.0) as u32;
        let scaled = if dst_w == src_w && dst_h == src_h {
            rgba
        } else {
            image::imageops::resize(&rgba, dst_w, dst_h, image::imageops::FilterType::Triangle)
        };

        Some(EmojiBitmap {
            width: scaled.width(),
            height: scaled.height(),
            rgba: scaled.into_raw(),
            draw_w,
            draw_h,
            left: origin_x as f32 * px_per_bitmap_px,
            bottom: origin_y as f32 * px_per_bitmap_px,
            advance: self.advance_px(gid, size),
        })
    }

    /// 读取某 strike 下字形的图片数据：(originOffsetX, originOffsetY, 图片字节)
    fn glyph_png(&self, strike: u64, gid: u16, depth: u8) -> Option<(i16, i16, Vec<u8>)> {
        if depth > 2 || gid >= self.num_glyphs {
            return None;
        }
        let mut file = self.file.lock().ok()?;
        // strike 头部：ppem(2) + ppi(2)，随后是 numGlyphs+1 个 u32 偏移
        let bounds = read_at(&mut file, strike + 4 + gid as u64 * 4, 8).ok()?;
        let start = be_u32(&bounds, 0) as u64;
        let end = be_u32(&bounds, 4) as u64;
        if end <= start + 8 {
            return None; // 空字形
        }
        let len = (end - start) as usize;
        let data = read_at(&mut file, strike + start, len).ok()?;
        drop(file);

        let origin_x = be_i16(&data, 0);
        let origin_y = be_i16(&data, 2);
        let graphic = &data[4..8];
        match graphic {
            b"png " | b"jpg " | b"tiff" => Some((origin_x, origin_y, data[8..].to_vec())),
            // 'dupe'：数据是另一个字形号，复用其位图
            b"dupe" if data.len() >= 10 => {
                let alias = be_u16(&data, 8);
                self.glyph_png(strike, alias, depth + 1)
            }
            _ => None,
        }
    }
}

// ============================ 字节读取 ============================

fn read_at(file: &mut File, offset: u64, len: usize) -> Result<Vec<u8>, String> {
    if len == 0 {
        return Ok(Vec::new());
    }
    file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; len];
    file.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

fn be_u16(buf: &[u8], at: usize) -> u16 {
    if at + 2 > buf.len() {
        return 0;
    }
    u16::from_be_bytes([buf[at], buf[at + 1]])
}

fn be_i16(buf: &[u8], at: usize) -> i16 {
    be_u16(buf, at) as i16
}

fn be_u32(buf: &[u8], at: usize) -> u32 {
    if at + 4 > buf.len() {
        return 0;
    }
    u32::from_be_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]])
}

/// 解析 cmap：优先 format 12（含 BMP 外码点），回退 format 4 / 6。
fn parse_cmap(buf: &[u8]) -> Option<HashMap<u32, u16>> {
    let num_tables = be_u16(buf, 2) as usize;
    let mut best: Option<(u8, usize)> = None; // (优先级, 子表偏移)
    for i in 0..num_tables {
        let rec = 4 + i * 8;
        if rec + 8 > buf.len() {
            break;
        }
        let platform = be_u16(buf, rec);
        let encoding = be_u16(buf, rec + 2);
        let offset = be_u32(buf, rec + 4) as usize;
        if offset >= buf.len() {
            continue;
        }
        let format = be_u16(buf, offset);
        // 优先级：全码点 format 12 > BMP format 4 > format 6
        let rank = match (format, platform, encoding) {
            (12, _, _) => 3,
            (4, _, _) => 2,
            (6, _, _) => 1,
            _ => 0,
        };
        if rank == 0 {
            continue;
        }
        if best.map(|(r, _)| rank > r).unwrap_or(true) {
            best = Some((rank, offset));
        }
    }
    let (_, offset) = best?;
    let mut map = HashMap::new();
    match be_u16(buf, offset) {
        12 => {
            let num_groups = be_u32(buf, offset + 12) as usize;
            for g in 0..num_groups {
                let at = offset + 16 + g * 12;
                if at + 12 > buf.len() {
                    break;
                }
                let start = be_u32(buf, at);
                let end = be_u32(buf, at + 4);
                let start_gid = be_u32(buf, at + 8);
                if end < start || end - start > 0x10_000 {
                    continue;
                }
                for (i, cp) in (start..=end).enumerate() {
                    map.insert(cp, (start_gid + i as u32) as u16);
                }
            }
        }
        4 => {
            let seg_count = (be_u16(buf, offset + 6) / 2) as usize;
            let ends = offset + 14;
            let starts = ends + seg_count * 2 + 2;
            let deltas = starts + seg_count * 2;
            let ranges = deltas + seg_count * 2;
            for s in 0..seg_count {
                let end = be_u16(buf, ends + s * 2) as u32;
                let start = be_u16(buf, starts + s * 2) as u32;
                let delta = be_u16(buf, deltas + s * 2);
                let range_off = be_u16(buf, ranges + s * 2) as usize;
                if start > end || end == 0xFFFF && start == 0xFFFF {
                    continue;
                }
                for cp in start..=end {
                    let gid = if range_off == 0 {
                        (cp as u16).wrapping_add(delta)
                    } else {
                        let at = ranges + s * 2 + range_off + (cp - start) as usize * 2;
                        let g = be_u16(buf, at);
                        if g == 0 {
                            continue;
                        }
                        g.wrapping_add(delta)
                    };
                    if gid != 0 {
                        map.insert(cp, gid);
                    }
                }
            }
        }
        6 => {
            let first = be_u16(buf, offset + 6) as u32;
            let count = be_u16(buf, offset + 8) as usize;
            for i in 0..count {
                let gid = be_u16(buf, offset + 10 + i * 2);
                if gid != 0 {
                    map.insert(first + i as u32, gid);
                }
            }
        }
        _ => return None,
    }
    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}
