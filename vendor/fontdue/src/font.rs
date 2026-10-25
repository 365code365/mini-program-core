use crate::layout::GlyphRasterConfig;
use crate::math::{Geometry, Line};
use crate::platform::{as_i32, ceil, floor, fract, is_negative};
use crate::raster::Raster;
use crate::table::{load_gsub, TableKern};
use crate::unicode;
use crate::FontResult;
use crate::{HashMap, HashSet};
use alloc::string::String;
use alloc::vec;
use alloc::vec::*;
use core::hash::{Hash, Hasher};
use core::mem;
use core::num::NonZeroU16;
use core::ops::Deref;
use ttf_parser::{Face, FaceParsingError, GlyphId, Tag};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Defines the bounds for a glyph's outline in subpixels. A glyph's outline is always contained in
/// its bitmap.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct OutlineBounds {
    /// Subpixel offset of the left-most edge of the glyph's outline.
    pub xmin: f32,
    /// Subpixel offset of the bottom-most edge of the glyph's outline.
    pub ymin: f32,
    /// The width of the outline in subpixels.
    pub width: f32,
    /// The height of the outline in subpixels.
    pub height: f32,
}

impl Default for OutlineBounds {
    fn default() -> Self {
        Self {
            xmin: 0.0,
            ymin: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }
}

impl OutlineBounds {
    /// Scales the bounding box by the given factor.
    #[inline(always)]
    pub fn scale(&self, scale: f32) -> OutlineBounds {
        OutlineBounds {
            xmin: self.xmin * scale,
            ymin: self.ymin * scale,
            width: self.width * scale,
            height: self.height * scale,
        }
    }
}

/// Encapsulates all layout information associated with a glyph for a fixed scale.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Metrics {
    /// Whole pixel offset of the left-most edge of the bitmap. This may be negative to reflect the
    /// glyph is positioned to the left of the origin.
    pub xmin: i32,
    /// Whole pixel offset of the bottom-most edge of the bitmap. This may be negative to reflect
    /// the glyph is positioned below the baseline.
    pub ymin: i32,
    /// The width of the bitmap in whole pixels.
    pub width: usize,
    /// The height of the bitmap in whole pixels.
    pub height: usize,
    /// Advance width of the glyph in subpixels. Used in horizontal fonts.
    pub advance_width: f32,
    /// Advance height of the glyph in subpixels. Used in vertical fonts.
    pub advance_height: f32,
    /// The bounding box that contains the glyph's outline at the offsets specified by the font.
    /// This is always a smaller box than the bitmap bounds.
    pub bounds: OutlineBounds,
}

impl Default for Metrics {
    fn default() -> Self {
        Metrics {
            xmin: 0,
            ymin: 0,
            width: 0,
            height: 0,
            advance_width: 0.0,
            advance_height: 0.0,
            bounds: OutlineBounds::default(),
        }
    }
}

/// Metrics associated with line positioning.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct LineMetrics {
    /// The highest point that any glyph in the font extends to above the baseline. Typically
    /// positive.
    pub ascent: f32,
    /// The lowest point that any glyph in the font extends to below the baseline. Typically
    /// negative.
    pub descent: f32,
    /// The gap to leave between the descent of one line and the ascent of the next. This is of
    /// course only a guideline given by the font's designers.
    pub line_gap: f32,
    /// A precalculated value for the height or width of the line depending on if the font is laid
    /// out horizontally or vertically. It's calculated by: ascent - descent + line_gap.
    pub new_line_size: f32,
}

impl LineMetrics {
    /// Creates a new line metrics struct and computes the new line size.
    fn new(ascent: i16, descent: i16, line_gap: i16) -> LineMetrics {
        // Operations between this values can exceed i16, so we extend to i32 here.
        let (ascent, descent, line_gap) = (ascent as i32, descent as i32, line_gap as i32);
        LineMetrics {
            ascent: ascent as f32,
            descent: descent as f32,
            line_gap: line_gap as f32,
            new_line_size: (ascent - descent + line_gap) as f32,
        }
    }

    /// Scales the line metrics by the given factor.
    #[inline(always)]
    fn scale(&self, scale: f32) -> LineMetrics {
        LineMetrics {
            ascent: self.ascent * scale,
            descent: self.descent * scale,
            line_gap: self.line_gap * scale,
            new_line_size: self.new_line_size * scale,
        }
    }
}

/// Stores compiled geometry and metric information.
#[derive(Clone)]
pub(crate) struct Glyph {
    pub v_lines: Vec<Line>,
    pub m_lines: Vec<Line>,
    advance_width: f32,
    advance_height: f32,
    pub bounds: OutlineBounds,
}

impl Default for Glyph {
    fn default() -> Self {
        Glyph {
            v_lines: Vec::new(),
            m_lines: Vec::new(),
            advance_width: 0.0,
            advance_height: 0.0,
            bounds: OutlineBounds::default(),
        }
    }
}

/// Settings for controlling specific font and layout behavior.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct FontSettings {
    /// The default is 0. The index of the font to use if parsing a font collection.
    pub collection_index: u32,
    /// The default is 40. The scale in px the font geometry is optimized for. Fonts rendered at
    /// the scale defined here will be the most optimal in terms of looks and performance. Glyphs
    /// rendered smaller than this scale will look the same but perform slightly worse, while
    /// glyphs rendered larger than this will looks worse but perform slightly better. The units of
    /// the scale are pixels per Em unit.
    pub scale: f32,
    /// The default is true. If enabled, will load glyphs for substitutions (liagtures, etc.) from
    /// the gsub table on compatible fonts. Only makes a difference when using indexed operations,
    /// i.e. `Font::raserize_indexed`, as singular characters do not have enough context to be
    /// substituted.
    pub load_substitutions: bool,
}

impl Default for FontSettings {
    fn default() -> FontSettings {
        FontSettings {
            collection_index: 0,
            scale: 40.0,
            load_substitutions: true,
        }
    }
}

/// 字体文件的字节：可以是读进来的 `Vec`、内嵌的 `&'static [u8]`，也可以是 mmap（文件页、可回收）。
/// 同一个文件的几个字面（TTC）共用一份
pub type FontBytes = Arc<dyn AsRef<[u8]> + Send + Sync>;

/// 一个字面最多留这么多字节的轮廓线段（一个汉字约 4~5KB，≈ 200 个字）。引擎的位图缓存按（字、字号）存，
/// 线段只在第一次画某个字号时用，所以留得少也不会反复展开
const GEOMETRY_BUDGET: usize = 1 << 20;

/// [`Font::bmp`] 里「还没查过」的记号（字形编号最大 65534，用不到它）
const UNKNOWN: u16 = u16::MAX;

#[derive(Default)]
struct GeometryCache {
    map: HashMap<u16, Arc<Glyph>>,
    order: std::collections::VecDeque<u16>,
    bytes: usize,
}

fn glyph_bytes(g: &Glyph) -> usize {
    (g.v_lines.capacity() + g.m_lines.capacity()) * core::mem::size_of::<Line>()
}

/// Represents a font. Fonts are immutable after creation.
///
/// **mini-render 的改动（惰性字形）**：上游在 `from_bytes` 里把 cmap 能到的**每个**字形的几何都展开好
/// （29352 字形的 CJK 字面 ≈ 305MB），这里改成第一次用到某个字形时才展开（[`Font::glyph`]），
/// 展开的代码、参数、顺序与上游逐行相同，所以同一个字形的度量与光栅化结果逐位一致。
/// 为此字体要留着文件字节（[`FontBytes`]）与解析好的 `Face`。
pub struct Font {
    name: Option<String>,
    units_per_em: f32,
    /// 每个字形一格，第一次用到时算度量（前进宽、包围盒）。格子里**不留**轮廓线段（`Box` 让没用到的格子只占一个指针）
    glyphs: Vec<OnceLock<Box<Glyph>>>,
    /// 轮廓线段（光栅化要用），按字节封顶（[`GEOMETRY_BUDGET`]），超了先进先出丢掉，再用到重新展开
    geometry: Mutex<GeometryCache>,
    /// BMP 字符 → 字形的查表结果（[`UNKNOWN`] = 还没查过）。查一次 cmap 记下来，热路径上只是一次原子读
    bmp: Box<[AtomicU16]>,
    /// BMP 以外的字符（emoji 之类）
    astral: Mutex<HashMap<char, u16>>,
    /// 上游的「全部字符 → 字形」表：只有调用 [`Font::chars`] 才建（引擎从来不调）
    all_chars: OnceLock<HashMap<char, NonZeroU16>>,
    /// 上游会展开的那些字形（cmap 映射到的 + gsub 里的 + 0 号）：别的下标与上游一样给空字形
    loadable: Vec<u64>,
    /// 解析好的字体表。借的是 [`Font::data`]：字段顺序保证它先于 data 释放
    face: Face<'static>,
    data: FontBytes,
    horizontal_line_metrics: Option<LineMetrics>,
    horizontal_kern: Option<HashMap<u32, i16>>,
    vertical_line_metrics: Option<LineMetrics>,
    settings: FontSettings,
    hash: OnceLock<usize>,
}

impl Hash for Font {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.file_hash().hash(state);
    }
}

impl core::fmt::Debug for Font {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Font")
            .field("name", &self.name)
            .field("settings", &self.settings)
            .field("units_per_em", &self.units_per_em)
            .field("hash", &self.hash)
            .finish()
    }
}

/// Converts a ttf-parser FaceParsingError into a string.
fn convert_error(error: FaceParsingError) -> &'static str {
    use FaceParsingError::*;
    match error {
        MalformedFont => "An attempt to read out of bounds detected.",
        UnknownMagic => "Face data must start with 0x00010000, 0x74727565, 0x4F54544F or 0x74746366.",
        FaceIndexOutOfBounds => "The face index is larger than the number of faces in the font.",
        NoHeadTable => "The head table is missing or malformed.",
        NoHheaTable => "The hhea table is missing or malformed.",
        NoMaxpTable => "The maxp table is missing or malformed.",
    }
}

fn convert_name(face: &Face) -> Option<String> {
    for name in face.names() {
        if name.name_id == 4 && name.is_unicode() {
            return Some(unicode::decode_utf16(name.name));
        }
    }
    None
}

impl Font {
    /// Constructs a font from an array of bytes.
    pub fn from_bytes<Data: Deref<Target = [u8]>>(data: Data, settings: FontSettings) -> FontResult<Font> {
        // 上游的签名：字节借来用完就还。惰性展开要留着字节，所以这里拷一份
        let owned: FontBytes = Arc::new(data.to_vec());
        Self::from_shared(owned, settings)
    }

    /// 与 [`from_bytes`](Self::from_bytes) 相同，但直接留着调用方给的字节（不拷）：mmap 的文件、内嵌的字体，
    /// 或同一个 TTC 的几个字面共用一份
    pub fn from_shared(data: FontBytes, settings: FontSettings) -> FontResult<Font> {
        // SAFETY: `face` 借 `data` 里的字节。`data` 是 Arc，与 `face` 存在同一个 Font 里，
        // 字段顺序让 `face` 先释放；`AsRef<[u8]>` 的实现（Vec / mmap / 'static 切片）在 Arc 活着时地址不变
        let bytes: &'static [u8] = unsafe { core::mem::transmute::<&[u8], &'static [u8]>((*data).as_ref()) };
        // 整个文件的哈希留到真有人问（`file_hash` / `Hash`）时再算：现算要把文件每一页都读一遍，
        // mmap 进来的字体就整个进了物理内存（Hiragino 23MB），而引擎从来不问它
        let hash = OnceLock::new();

        let face = match Face::parse(bytes, settings.collection_index) {
            Ok(f) => f,
            Err(e) => return Err(convert_error(e)),
        };
        let name = convert_name(&face);

        // Optionally get kerning values for the font. This should be a try block in the future.
        let horizontal_kern: Option<HashMap<u32, i16>> = (|| {
            let table: &[u8] = face.raw_face().table(Tag::from_bytes(&b"kern"))?;
            let table: TableKern = TableKern::new(table)?;
            Some(table.horizontal_mappings)
        })();

        // Collect all the unique codepoint to glyph mappings.
        let glyph_count = face.number_of_glyphs();
        let mut indices_to_load = HashSet::with_capacity(glyph_count as usize);
        indices_to_load.insert(0u16);
        // 字符 → 字形表不在这里建（三万项 ≈ 1MB / 字面），用到哪个字查哪个（[`Font::lookup_glyph_index`]），
        // 查法与这里的「后面的子表覆盖前面的」一致。这一遍只记下哪些字形是上游会展开的
        if let Some(subtable) = face.tables().cmap {
            for subtable in subtable.subtables {
                subtable.codepoints(|codepoint| {
                    if let Some(mapping) = subtable.glyph_index(codepoint) {
                        if let Some(mapping) = NonZeroU16::new(mapping.0) {
                            indices_to_load.insert(mapping.get());
                        }
                    }
                })
            }
        }

        // If the gsub table exists and the user needs it, add all of its glyphs to the glyphs we should load.
        if settings.load_substitutions {
            load_gsub(&face, &mut indices_to_load);
        }

        let units_per_em = face.units_per_em() as f32;

        // 不在这里展开：只记下上游会展开哪些（[`Font::glyph`] 用到时才展开）。
        // 上游对越界下标报错（整个 from_bytes 失败），这里照样在加载时检查
        let mut loadable = vec![0u64; (glyph_count as usize + 63) / 64];
        for index in indices_to_load {
            if index >= glyph_count {
                return Err("Attempted to map a codepoint out of bounds.");
            }
            loadable[index as usize / 64] |= 1u64 << (index % 64);
        }
        let mut glyphs: Vec<OnceLock<Box<Glyph>>> = Vec::with_capacity(glyph_count as usize);
        glyphs.resize_with(glyph_count as usize, OnceLock::new);

        // New line metrics.
        let horizontal_line_metrics =
            Some(LineMetrics::new(face.ascender(), face.descender(), face.line_gap()));
        let vertical_line_metrics = if let Some(ascender) = face.vertical_ascender() {
            Some(LineMetrics::new(
                ascender,
                face.vertical_descender().unwrap_or(0),
                face.vertical_line_gap().unwrap_or(0),
            ))
        } else {
            None
        };

        Ok(Font {
            name,
            glyphs,
            loadable,
            geometry: Mutex::new(GeometryCache::default()),
            bmp: (0..0x10000).map(|_| AtomicU16::new(UNKNOWN)).collect(),
            astral: Mutex::new(HashMap::new()),
            all_chars: OnceLock::new(),
            face,
            data,
            units_per_em,
            horizontal_line_metrics,
            horizontal_kern,
            vertical_line_metrics,
            settings,
            hash,
        })
    }

    /// 第 `index` 个字形（第一次用到时展开，与上游 `from_bytes` 里的 `generate_glyph` 逐行相同）。
    /// 上游没展开的字形（cmap / gsub 都到不了）给空字形，与上游一致；越界照旧 panic
    fn glyph(&self, index: u16) -> &Glyph {
        self.glyphs[index as usize].get_or_init(|| {
            let full = self.expand(index);
            // 格子里只留度量；线段交给封顶的几何缓存
            let metrics = Glyph { v_lines: Vec::new(), m_lines: Vec::new(), ..full };
            self.keep_geometry(index, Arc::new(full));
            Box::new(metrics)
        })
    }

    /// 光栅化要的完整字形（含轮廓线段）：几何缓存里有就拿，没有就重新展开（结果与第一次逐位相同）
    fn geometry(&self, index: u16) -> Arc<Glyph> {
        self.glyph(index);
        if let Some(g) = self.geometry.lock().ok().and_then(|c| c.map.get(&index).cloned()) {
            return g;
        }
        let g = Arc::new(self.expand(index));
        self.keep_geometry(index, g.clone());
        g
    }

    fn keep_geometry(&self, index: u16, g: Arc<Glyph>) {
        let Ok(mut c) = self.geometry.lock() else { return };
        let bytes = glyph_bytes(&g);
        if c.map.insert(index, g).is_none() {
            c.order.push_back(index);
            c.bytes += bytes;
        }
        while c.bytes > GEOMETRY_BUDGET && c.order.len() > 1 {
            let Some(old) = c.order.pop_front() else { break };
            if let Some(g) = c.map.remove(&old) {
                c.bytes -= glyph_bytes(&g);
            }
        }
    }

    /// 展开第 `index` 个字形（与上游 `from_bytes` 里的 `generate_glyph` 逐行相同）。
    /// 上游没展开的字形（cmap / gsub 都到不了）给空字形
    fn expand(&self, index: u16) -> Glyph {
        {
            let mut glyph = Glyph::default();
            if self.loadable[index as usize / 64] & (1u64 << (index % 64)) == 0 {
                return glyph;
            }
            let face = &self.face;
            let glyph_id = GlyphId(index);
            if let Some(advance_width) = face.glyph_hor_advance(glyph_id) {
                glyph.advance_width = advance_width as f32;
            }
            if let Some(advance_height) = face.glyph_ver_advance(glyph_id) {
                glyph.advance_height = advance_height as f32;
            }
            let mut geometry = Geometry::new(self.settings.scale, self.units_per_em);
            face.outline_glyph(glyph_id, &mut geometry);
            geometry.finalize(&mut glyph);
            glyph
        }
    }

    /// 已经展开了几何的字形个数（诊断用：惰性展开省了多少）
    pub fn loaded_glyph_count(&self) -> usize {
        self.glyphs.iter().filter(|g| g.get().is_some()).count()
    }

    /// 这个字面在堆上大约占多少字节：(展开的字形几何, 字符 → 字形表与每字形一格的槽位)。
    /// 不含字体文件本身（mmap / 内嵌，见 [`FontBytes`]）
    pub fn heap_bytes(&self) -> (usize, usize) {
        let line = core::mem::size_of::<Line>();
        let geometry: usize = self
            .glyphs
            .iter()
            .filter_map(|g| g.get())
            .map(|g| core::mem::size_of::<Glyph>() + (g.v_lines.capacity() + g.m_lines.capacity()) * line)
            .sum();
        let geometry = geometry + self.geometry.lock().map(|c| c.bytes).unwrap_or(0);
        let map = self.bmp.len() * 2 + self.astral.lock().map(|m| m.capacity() * 8).unwrap_or(0);
        let slots = self.glyphs.capacity() * core::mem::size_of::<OnceLock<Box<Glyph>>>() + self.loadable.capacity() * 8;
        (geometry, map + slots)
    }

    /// 字体文件的字节（同一个 TTC 的其它字面可以用 [`from_shared`](Self::from_shared) 共用）
    pub fn bytes(&self) -> FontBytes {
        self.data.clone()
    }

    /// Returns the font's face name if it has one. It is from `Name ID 4` (Full Name) in the name table.
    /// See https://learn.microsoft.com/en-us/typography/opentype/spec/name#name-ids for more info.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns all valid unicode codepoints that have mappings to glyph geometry in the font, along
    /// with their associated index. This does not include grapheme cluster mappings. The mapped
    /// NonZeroU16 index can be used in the _indexed font functions.
    pub fn chars(&self) -> &HashMap<char, NonZeroU16> {
        self.all_chars.get_or_init(|| {
            let mut m = HashMap::new();
            if let Some(cmap) = self.face.tables().cmap {
                for subtable in cmap.subtables {
                    subtable.codepoints(|codepoint| {
                        if let Some(mapping) = subtable.glyph_index(codepoint) {
                            if let Some(mapping) = NonZeroU16::new(mapping.0) {
                                m.insert(unsafe { mem::transmute::<u32, char>(codepoint) }, mapping);
                            }
                        }
                    })
                }
            }
            m
        })
    }

    /// Returns a precomputed hash for the font file.
    pub fn file_hash(&self) -> usize {
        *self.hash.get_or_init(|| crate::hash::hash((*self.data).as_ref()))
    }

    /// New line metrics for fonts that append characters to lines horizontally, and append new
    /// lines vertically (above or below the current line). Only populated for fonts with the
    /// appropriate metrics, none if it's missing.
    /// # Arguments
    ///
    /// * `px` - The size to scale the line metrics by. The units of the scale are pixels per Em
    /// unit.
    pub fn horizontal_line_metrics(&self, px: f32) -> Option<LineMetrics> {
        let metrics = self.horizontal_line_metrics?;
        Some(metrics.scale(self.scale_factor(px)))
    }

    /// New line metrics for fonts that append characters to lines vertically, and append new
    /// lines horizontally (left or right of the current line). Only populated for fonts with the
    /// appropriate metrics, none if it's missing.
    /// # Arguments
    ///
    /// * `px` - The size to scale the line metrics by. The units of the scale are pixels per Em
    /// unit.
    pub fn vertical_line_metrics(&self, px: f32) -> Option<LineMetrics> {
        let metrics = self.vertical_line_metrics?;
        Some(metrics.scale(self.scale_factor(px)))
    }

    /// Gets the font's units per em.
    #[inline(always)]
    pub fn units_per_em(&self) -> f32 {
        self.units_per_em
    }

    /// Calculates the glyph's outline scale factor for a given px size. The units of the scale are
    /// pixels per Em unit.
    #[inline(always)]
    pub fn scale_factor(&self, px: f32) -> f32 {
        px / self.units_per_em
    }

    /// Retrieves the horizontal scaled kerning value for two adjacent characters.
    /// # Arguments
    ///
    /// * `left` - The character on the left hand side of the pairing.
    /// * `right` - The character on the right hand side of the pairing.
    /// * `px` - The size to scale the kerning value for. The units of the scale are pixels per Em
    /// unit.
    /// # Returns
    ///
    /// * `Option<f32>` - The horizontal scaled kerning value if one is present in the font for the
    /// given left and right pair, None otherwise.
    #[inline(always)]
    pub fn horizontal_kern(&self, left: char, right: char, px: f32) -> Option<f32> {
        self.horizontal_kern_indexed(self.lookup_glyph_index(left), self.lookup_glyph_index(right), px)
    }

    /// Retrieves the horizontal scaled kerning value for two adjacent glyph indicies.
    /// # Arguments
    ///
    /// * `left` - The glyph index on the left hand side of the pairing.
    /// * `right` - The glyph index on the right hand side of the pairing.
    /// * `px` - The size to scale the kerning value for. The units of the scale are pixels per Em
    /// unit.
    /// # Returns
    ///
    /// * `Option<f32>` - The horizontal scaled kerning value if one is present in the font for the
    /// given left and right pair, None otherwise.
    #[inline(always)]
    pub fn horizontal_kern_indexed(&self, left: u16, right: u16, px: f32) -> Option<f32> {
        let scale = self.scale_factor(px);
        let map = self.horizontal_kern.as_ref()?;
        let key = u32::from(left) << 16 | u32::from(right);
        let value = map.get(&key)?;
        Some((*value as f32) * scale)
    }

    /// Retrieves the layout metrics for the given character. If the character isn't present in the
    /// font, then the layout for the font's default character is returned instead.
    /// # Arguments
    ///
    /// * `index` - The character in the font to to generate the layout metrics for.
    /// * `px` - The size to generate the layout metrics for the character at. Cannot be negative.
    /// The units of the scale are pixels per Em unit.
    /// # Returns
    ///
    /// * `Metrics` - Sizing and positioning metadata for the glyph.
    #[inline]
    pub fn metrics(&self, character: char, px: f32) -> Metrics {
        self.metrics_indexed(self.lookup_glyph_index(character), px)
    }

    /// Retrieves the layout metrics at the given index. You normally want to be using
    /// metrics(char, f32) instead, unless your glyphs are pre-indexed.
    /// # Arguments
    ///
    /// * `index` - The glyph index in the font to to generate the layout metrics for.
    /// * `px` - The size to generate the layout metrics for the glyph at. Cannot be negative. The
    /// units of the scale are pixels per Em unit.
    /// # Returns
    ///
    /// * `Metrics` - Sizing and positioning metadata for the glyph.
    pub fn metrics_indexed(&self, index: u16, px: f32) -> Metrics {
        let glyph = self.glyph(index);
        let scale = self.scale_factor(px);
        let (metrics, _, _) = self.metrics_raw(scale, glyph, 0.0);
        metrics
    }

    /// Internal function to generate the metrics, offset_x, and offset_y of the glyph.
    fn metrics_raw(&self, scale: f32, glyph: &Glyph, offset: f32) -> (Metrics, f32, f32) {
        let bounds = glyph.bounds.scale(scale);
        let mut offset_x = fract(bounds.xmin + offset);
        let mut offset_y = fract(1.0 - fract(bounds.height) - fract(bounds.ymin));
        if is_negative(offset_x) {
            offset_x += 1.0;
        }
        if is_negative(offset_y) {
            offset_y += 1.0;
        }
        let metrics = Metrics {
            xmin: as_i32(floor(bounds.xmin)),
            ymin: as_i32(floor(bounds.ymin)),
            width: as_i32(ceil(bounds.width + offset_x)) as usize,
            height: as_i32(ceil(bounds.height + offset_y)) as usize,
            advance_width: scale * glyph.advance_width,
            advance_height: scale * glyph.advance_height,
            bounds,
        };
        (metrics, offset_x, offset_y)
    }

    /// Retrieves the layout rasterized bitmap for the given raster config. If the raster config's
    /// character isn't present in the font, then the layout and bitmap for the font's default
    /// character's raster is returned instead.
    /// # Arguments
    ///
    /// * `config` - The settings to render the character at.
    /// # Returns
    ///
    /// * `Metrics` - Sizing and positioning metadata for the rasterized glyph.
    /// * `Vec<u8>` - Coverage vector for the glyph. Coverage is a linear scale where 0 represents
    /// 0% coverage of that pixel by the glyph and 255 represents 100% coverage. The vec starts at
    /// the top left corner of the glyph.
    #[inline]
    pub fn rasterize_config(&self, config: GlyphRasterConfig) -> (Metrics, Vec<u8>) {
        self.rasterize_indexed(config.glyph_index, config.px)
    }

    /// Retrieves the layout metrics and rasterized bitmap for the given character. If the
    /// character isn't present in the font, then the layout and bitmap for the font's default
    /// character is returned instead.
    /// # Arguments
    ///
    /// * `character` - The character to rasterize.
    /// * `px` - The size to render the character at. Cannot be negative. The units of the scale
    /// are pixels per Em unit.
    /// # Returns
    ///
    /// * `Metrics` - Sizing and positioning metadata for the rasterized glyph.
    /// * `Vec<u8>` - Coverage vector for the glyph. Coverage is a linear scale where 0 represents
    /// 0% coverage of that pixel by the glyph and 255 represents 100% coverage. The vec starts at
    /// the top left corner of the glyph.
    #[inline]
    pub fn rasterize(&self, character: char, px: f32) -> (Metrics, Vec<u8>) {
        self.rasterize_indexed(self.lookup_glyph_index(character), px)
    }

    /// Retrieves the layout rasterized bitmap for the given raster config. If the raster config's
    /// character isn't present in the font, then the layout and bitmap for the font's default
    /// character's raster is returned instead.
    ///
    /// This will perform the operation with the width multiplied by 3, as to simulate subpixels.
    /// Taking these as RGB values will perform subpixel anti aliasing.
    /// # Arguments
    ///
    /// * `config` - The settings to render the character at.
    /// # Returns
    ///
    /// * `Metrics` - Sizing and positioning metadata for the rasterized glyph.
    /// * `Vec<u8>` - Swizzled RGB coverage vector for the glyph. Coverage is a linear scale where 0
    /// represents 0% coverage of that subpixel by the glyph and 255 represents 100% coverage. The
    /// vec starts at the top left corner of the glyph.
    #[inline]
    pub fn rasterize_config_subpixel(&self, config: GlyphRasterConfig) -> (Metrics, Vec<u8>) {
        self.rasterize_indexed_subpixel(config.glyph_index, config.px)
    }

    /// Retrieves the layout metrics and rasterized bitmap for the given character. If the
    /// character isn't present in the font, then the layout and bitmap for the font's default
    /// character is returned instead.
    ///
    /// This will perform the operation with the width multiplied by 3, as to simulate subpixels.
    /// Taking these as RGB values will perform subpixel anti aliasing.
    /// # Arguments
    ///
    /// * `character` - The character to rasterize.
    /// * `px` - The size to render the character at. Cannot be negative. The units of the scale
    /// are pixels per Em unit.
    /// # Returns
    ///
    /// * `Metrics` - Sizing and positioning metadata for the rasterized glyph.
    /// * `Vec<u8>` - Swizzled RGB coverage vector for the glyph. Coverage is a linear scale where 0
    /// represents 0% coverage of that subpixel by the glyph and 255 represents 100% coverage. The
    /// vec starts at the top left corner of the glyph.
    #[inline]
    pub fn rasterize_subpixel(&self, character: char, px: f32) -> (Metrics, Vec<u8>) {
        self.rasterize_indexed_subpixel(self.lookup_glyph_index(character), px)
    }

    /// Retrieves the layout metrics and rasterized bitmap at the given index. You normally want to
    /// be using rasterize(char, f32) instead, unless your glyphs are pre-indexed.
    /// # Arguments
    ///
    /// * `index` - The glyph index in the font to rasterize.
    /// * `px` - The size to render the character at. Cannot be negative. The units of the scale
    /// are pixels per Em unit.
    /// # Returns
    ///
    /// * `Metrics` - Sizing and positioning metadata for the rasterized glyph.
    /// * `Vec<u8>` - Coverage vector for the glyph. Coverage is a linear scale where 0 represents
    /// 0% coverage of that pixel by the glyph and 255 represents 100% coverage. The vec starts at
    /// the top left corner of the glyph.
    pub fn rasterize_indexed(&self, index: u16, px: f32) -> (Metrics, Vec<u8>) {
        if px <= 0.0 {
            return (Metrics::default(), Vec::new());
        }
        let glyph = self.geometry(index);
        let scale = self.scale_factor(px);
        let (metrics, offset_x, offset_y) = self.metrics_raw(scale, &glyph, 0.0);
        let mut canvas = Raster::new(metrics.width, metrics.height);
        canvas.draw(&glyph, scale, scale, offset_x, offset_y);
        (metrics, canvas.get_bitmap())
    }

    /// Retrieves the layout metrics and rasterized bitmap at the given index. You normally want to
    /// be using rasterize(char, f32) instead, unless your glyphs are pre-indexed.
    ///
    /// This will perform the operation with the width multiplied by 3, as to simulate subpixels.
    /// Taking these as RGB values will perform subpixel anti aliasing.
    /// # Arguments
    ///
    /// * `index` - The glyph index in the font to rasterize.
    /// * `px` - The size to render the character at. Cannot be negative. The units of the scale
    /// are pixels per Em unit.
    /// # Returns
    ///
    /// * `Metrics` - Sizing and positioning metadata for the rasterized glyph.
    /// * `Vec<u8>` - Swizzled RGB coverage vector for the glyph. Coverage is a linear scale where 0
    /// represents 0% coverage of that subpixel by the glyph and 255 represents 100% coverage. The
    /// vec starts at the top left corner of the glyph.
    pub fn rasterize_indexed_subpixel(&self, index: u16, px: f32) -> (Metrics, Vec<u8>) {
        if px <= 0.0 {
            return (Metrics::default(), Vec::new());
        }
        let glyph = self.geometry(index);
        let scale = self.scale_factor(px);
        let (metrics, offset_x, offset_y) = self.metrics_raw(scale, &glyph, 0.0);
        let mut canvas = Raster::new(metrics.width * 3, metrics.height);
        canvas.draw(&glyph, scale * 3.0, scale, offset_x, offset_y);
        (metrics, canvas.get_bitmap())
    }

    /// Checks if the font has a glyph for the given character.
    #[inline]
    pub fn has_glyph(&self, character: char) -> bool {
        self.lookup_glyph_index(character) != 0
    }

    /// Finds the internal glyph index for the given character. If the character is not present in
    /// the font then 0 is returned.
    #[inline]
    pub fn lookup_glyph_index(&self, character: char) -> u16 {
        let c = character as u32;
        if c < 0x10000 {
            let slot = &self.bmp[c as usize];
            let v = slot.load(Ordering::Relaxed);
            if v != UNKNOWN {
                return v;
            }
            let g = self.cmap_lookup(c);
            slot.store(g, Ordering::Relaxed);
            return g;
        }
        if let Some(g) = self.astral.lock().ok().and_then(|m| m.get(&character).copied()) {
            return g;
        }
        let g = self.cmap_lookup(c);
        if let Ok(mut m) = self.astral.lock() {
            m.insert(character, g);
        }
        g
    }

    /// 上游建表的规则（按子表顺序逐个插入、后插的覆盖先插的、映射到 0 的不插）倒过来查：
    /// 从最后一个子表往前，第一个把它映射到非 0 字形的就是答案
    fn cmap_lookup(&self, c: u32) -> u16 {
        let Some(cmap) = self.face.tables().cmap else { return 0 };
        let st = cmap.subtables;
        // 只查 Unicode 子表（平台 0、或 Windows 的 Unicode BMP / 全集）。mini-render 的改动：上游连 Mac 平台的
        // 旧编码表一起查 —— 冬青黑（Hiragino Sans GB）带一张 (1, 25) 简体中文 GB 的 format 2 表，Unicode 表里没有的
        // U+203A「›」被它当成双字节码查，高字节 0x20 落到单字节区，取回的是 0x3A「:」的字形：
        // 「持仓 ›」画成「持仓 :」。字体里一张 Unicode 子表都没有时才退回全部子表（与上游一样）
        let has_unicode = (0..st.len()).any(|i| st.get(i).is_some_and(|t| t.is_unicode()));
        for i in (0..st.len()).rev() {
            let Some(t) = st.get(i) else { continue };
            if has_unicode && !t.is_unicode() {
                continue;
            }
            if let Some(g) = t.glyph_index(c) {
                if g.0 != 0 {
                    return g.0;
                }
            }
        }
        0
    }

    /// Gets the total glyphs in the font.
    pub fn glyph_count(&self) -> u16 {
        self.glyphs.len() as u16
    }
}
