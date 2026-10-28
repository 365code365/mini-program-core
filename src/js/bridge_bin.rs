//! 走二进制桥的原生函数（`register_raw`，参数与返回是 JS 原值，见 js/binary.rs）。
//!
//! | 入口 | 给谁 |
//! |---|---|
//! | `__native_text_encode(str, format)` → ArrayBuffer | `wx.encode`（utf8 / ucs2 / utf16le / latin1 / gbk） |
//! | `__native_text_decode(bytes, format)` → string | `wx.decode` |
//! | `__native_b64_encode(bytes)` → string | `wx.arrayBufferToBase64`（小游戏） |
//! | `__native_b64_decode(str)` → ArrayBuffer | `wx.base64ToArrayBuffer`（小游戏） |
//!
//! 小程序与小游戏都注册（契约测试要求注册的都有人用：现在只有小游戏的 game-compat.js 用）；
//! 小程序的 codec.js 换成原生是行为变化，单独做。

use super::binary::{array_buffer, require_bytes};
use super::JsRuntime;
use base64::Engine as _;
use rquickjs::{Exception, IntoJs};

/// 统一编码名（官方合法值：utf8 / utf-8 / ucs2 / ucs-2 / utf16le / utf-16le / latin1 / gbk），不认识的返回 None
fn norm(format: &str) -> Option<&'static str> {
    Some(match format.to_ascii_lowercase().as_str() {
        "" | "utf8" | "utf-8" => "utf8",
        "ucs2" | "ucs-2" | "utf16le" | "utf-16le" => "utf16le",
        "latin1" => "latin1",
        "gbk" => "gbk",
        _ => return None,
    })
}

pub(super) fn encode(s: &str, format: &str) -> Option<Vec<u8>> {
    Some(match norm(format)? {
        "utf8" => s.as_bytes().to_vec(),
        "utf16le" => s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect(),
        // latin1：每个 UTF-16 码元取低 8 位（与 Node Buffer.from(s, 'latin1') 一致）
        "latin1" => s.encode_utf16().map(|u| u as u8).collect(),
        _ => encoding_rs::GBK.encode(s).0.into_owned(),
    })
}

pub(super) fn decode(b: &[u8], format: &str) -> Option<String> {
    Some(match norm(format)? {
        // 非法序列替换成 U+FFFD（与 TextDecoder 默认的非 fatal 行为一致）
        "utf8" => String::from_utf8_lossy(b).into_owned(),
        "utf16le" => {
            let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units)
        }
        "latin1" => b.iter().map(|&c| c as char).collect(),
        _ => encoding_rs::GBK.decode_without_bom_handling(b).0.into_owned(),
    })
}

pub(super) fn register(rt: &JsRuntime) -> Result<(), String> {
    rt.register_raw("__native_text_encode", |ctx, args| {
        let s: String = args.0.first().and_then(|v| v.as_string()).and_then(|s| s.to_string().ok())
            .ok_or_else(|| Exception::throw_type(&ctx, "data 应当是字符串"))?;
        let format = args.0.get(1).and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
        let bytes = encode(&s, &format).ok_or_else(|| Exception::throw_type(&ctx, &format!("不支持的编码格式 {format}")))?;
        array_buffer(&ctx, bytes)
    })?;
    rt.register_raw("__native_text_decode", |ctx, args| {
        let format = args.0.get(1).and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
        norm(&format).ok_or_else(|| Exception::throw_type(&ctx, &format!("不支持的编码格式 {format}")))?;
        let s = require_bytes(&ctx, args.0.first(), "data", |b| decode(b, &format).unwrap_or_default())?;
        s.into_js(&ctx)
    })?;
    rt.register_raw("__native_b64_encode", |ctx, args| {
        let s = require_bytes(&ctx, args.0.first(), "arrayBuffer", |b| base64::engine::general_purpose::STANDARD.encode(b))?;
        s.into_js(&ctx)
    })?;
    rt.register_raw("__native_b64_decode", |ctx, args| {
        let s: String = args.0.first().and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
        // 宽松：忽略空白、补齐缺的 '='（与 codec.js 的老实现一致：微信里传带换行的 base64 也能解）
        let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        let engine = base64::engine::GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::GeneralPurposeConfig::new().with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
        );
        match engine.decode(clean.trim_end_matches('=')) {
            Ok(b) => array_buffer(&ctx, b),
            Err(e) => Err(Exception::throw_type(&ctx, &format!("base64 不合法：{e}"))),
        }
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[test]
    fn 各编码往返() {
        let s = "我爱 Beijing 天安门 😀";
        for f in ["utf8", "utf-8", "ucs2", "ucs-2", "utf16le", "utf-16le"] {
            assert_eq!(decode(&encode(s, f).unwrap(), f).unwrap(), s, "{f}");
        }
        // gbk 不能表示 emoji：去掉再比
        let g = "我爱 Beijing 天安门";
        assert_eq!(decode(&encode(g, "gbk").unwrap(), "gbk").unwrap(), g);
        assert_eq!(encode("我", "gbk").unwrap(), vec![0xCE, 0xD2]);
        assert_eq!(encode("é", "latin1").unwrap(), vec![0xE9]);
        assert_eq!(decode(&[0xE9], "latin1").unwrap(), "é");
        assert!(encode("x", "ascii").is_none());
        assert_eq!(decode(&[0xff, 0x41], "utf8").unwrap(), "\u{FFFD}A", "非法序列替换为 U+FFFD");
    }
}
