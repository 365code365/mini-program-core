//! `wx.loadFont(path)` 与 `wx.getTextLineHeight(opts)`（game-doc/03、05 GT-F1 / F5）。
//!
//! - loadFont 同步：读字体文件的 name 表取字族名（nameID 1，优先英文），登记进 `text_faces`（与小程序 `wx.loadFontFace`
//!   同一张表，画布 `font` 里写这个字族就能用）；成功返回字族名，失败返回 null（官方）。只认 TTF / OTF / TTC；
//!   路径是包内路径（按包根，跑出包根拒绝）或 `wxfile://`；
//! - getTextLineHeight：`line-height: normal` 的行高，按该字族 / 字号与文字内容（含 CJK 与否）取，与 `<text>` 排版同一个口径。
//!   真机的计算口径待取证（F5，±1px）。

use crate::js::JsRuntime;
use rquickjs::{IntoJs, Value};
use std::path::{Path, PathBuf};

pub fn register(rt: &JsRuntime, root: PathBuf) -> Result<(), String> {
    rt.register_raw("__game_load_font", move |ctx, args| {
        let src = args.0.first().and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
        match load_font(&src, &root) {
            Ok(family) => family.into_js(&ctx),
            Err(e) => {
                if std::env::var_os("MINI_FONT_LOG").is_some() {
                    eprintln!("🔤 loadFont({src:?}) 失败：{e}");
                }
                Ok(Value::new_null(ctx))
            }
        }
    })?;
    rt.register_raw("__game_text_line_height", move |ctx, args| {
        let o: serde_json::Value = args
            .0
            .first()
            .and_then(|v| v.as_string())
            .and_then(|s| s.to_string().ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let size = o.get("fontSize").and_then(|v| v.as_f64()).filter(|v| *v > 0.0).unwrap_or(16.0) as f32;
        let family = o.get("fontFamily").and_then(|v| v.as_str()).unwrap_or("sans-serif");
        let text = o.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let h = crate::text_family::renderer_for_family(Some(family))
            .map(|tr| tr.natural_line_height_for(text, size))
            .unwrap_or(size * 1.375);
        (h.round() as f64).into_js(&ctx)
    })?;
    Ok(())
}

fn load_font(src: &str, root: &Path) -> Result<String, String> {
    let path = if src.starts_with(crate::wxfile::SCHEME) {
        crate::wxfile::to_path(src).ok_or("无效的 wxfile 路径")?
    } else {
        super::image::loader::package_path(src, root).ok_or("路径不能跑出包根")?
    };
    let bytes = std::fs::read(&path).map_err(|e| format!("读不出 {}：{e}", path.display()))?;
    let family = family_name(&bytes).unwrap_or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
    if family.is_empty() {
        return Err("字体没有字族名".into());
    }
    crate::text_faces::register_file(&family, &path)?;
    Ok(family)
}

/// name 表的字族名（nameID 1）：优先 Windows 英文（0x0409），其次 Mac 英文，再次任一 Unicode / Windows 记录。
/// 手解 name 表（几十行）而不引 ttf-parser：那个库在这里只是开发依赖，为一张表加一个发布依赖不值
fn family_name(b: &[u8]) -> Option<String> {
    let u16_at = |o: usize| b.get(o..o + 2).map(|x| u16::from_be_bytes([x[0], x[1]]));
    let u32_at = |o: usize| b.get(o..o + 4).map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]]));
    // TTC：取第一个字面
    let base = if b.get(..4) == Some(b"ttcf") { u32_at(12)? as usize } else { 0 };
    let n_tables = u16_at(base + 4)? as usize;
    let name = (0..n_tables).find_map(|i| {
        let r = base + 12 + i * 16;
        (b.get(r..r + 4) == Some(b"name")).then(|| u32_at(r + 8)).flatten()
    })? as usize;
    let count = u16_at(name + 2)? as usize;
    let strings = name + u16_at(name + 4)? as usize;
    let mut best: Option<(u8, String)> = None;
    for i in 0..count {
        let r = name + 6 + i * 12;
        let (plat, lang, id) = (u16_at(r)?, u16_at(r + 4)?, u16_at(r + 6)?);
        if id != 1 {
            continue;
        }
        let (len, off) = (u16_at(r + 8)? as usize, u16_at(r + 10)? as usize);
        let raw = b.get(strings + off..strings + off + len)?;
        let text = if plat == 1 {
            raw.iter().map(|&c| c as char).collect::<String>()
        } else {
            let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units)
        };
        let rank = match (plat, lang) {
            (3, 0x0409) => 0,
            (1, 0) => 1,
            _ => 2,
        };
        if !text.trim().is_empty() && best.as_ref().is_none_or(|(r0, _)| rank < *r0) {
            best = Some((rank, text.trim().to_string()));
        }
    }
    best.map(|(_, t)| t)
}

#[cfg(test)]
mod tests {
    #[test]
    fn 字族名取自_name_表() {
        let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/Arial.ttf")).unwrap();
        assert_eq!(super::family_name(&bytes).as_deref(), Some("Arial"));
        assert_eq!(super::family_name(b"not a font"), None);
    }
}
