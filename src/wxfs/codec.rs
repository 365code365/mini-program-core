//! 文件内容的编码（`encoding` 参数）与摘要（`getFileInfo` 的 md5 / sha1）。
//!
//! 摘要自己实现而不是引一个 crate：两种算法加起来不到百行、只用在 `getFileInfo` 上，
//! 不值得为它多编两个依赖；正确性由下面的 RFC 测试向量守着。

use base64::Engine as _;

/// 微信认的编码名（官方 FileSystemManager.readFile 的 encoding 表），统一成小写短名
fn norm(enc: &str) -> String {
    enc.trim().to_ascii_lowercase().replace('-', "")
}

/// 字节 → 字符串。`None` 表示编码名不认识
pub fn decode(bytes: &[u8], enc: &str) -> Option<String> {
    Some(match norm(enc).as_str() {
        "utf8" => String::from_utf8_lossy(bytes).into_owned(),
        "base64" => base64::engine::general_purpose::STANDARD.encode(bytes),
        "hex" => bytes.iter().map(|b| format!("{b:02x}")).collect(),
        "ascii" => bytes.iter().map(|b| (b & 0x7f) as char).collect(),
        "binary" | "latin1" => bytes.iter().map(|&b| b as char).collect(),
        "ucs2" | "utf16le" => {
            let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            String::from_utf16_lossy(&units)
        }
        _ => return None,
    })
}

/// 字符串 → 字节（写文件）。`None` 表示编码名不认识或内容不合法（坏的 base64 / hex）
pub fn encode(s: &str, enc: &str) -> Option<Vec<u8>> {
    Some(match norm(enc).as_str() {
        "" | "utf8" => s.as_bytes().to_vec(),
        "base64" => base64::engine::general_purpose::STANDARD
            .decode(s.trim().as_bytes())
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(s.trim().trim_end_matches('=').as_bytes()))
            .ok()?,
        "hex" => {
            let s = s.trim();
            if s.len() % 2 != 0 {
                return None;
            }
            (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect::<Option<Vec<_>>>()?
        }
        "ascii" | "binary" | "latin1" => s.chars().map(|c| c as u32 as u8).collect(),
        "ucs2" | "utf16le" => s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect(),
        _ => return None,
    })
}

pub fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub fn unb64(s: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::STANDARD.decode(s.as_bytes()).ok()
}

/// MD5（RFC 1321），小写十六进制
pub fn md5_hex(data: &[u8]) -> String {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32).collect();
    let (mut a0, mut b0, mut c0, mut d0) = (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());
    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk.chunks(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    [a0, b0, c0, d0].iter().flat_map(|w| w.to_le_bytes()).map(|b| format!("{b:02x}")).collect()
}

/// SHA-1（RFC 3174），小写十六进制
pub fn sha1_hex(data: &[u8]) -> String {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i / 20 {
                0 => ((b & c) | (!b & d), 0x5A827999),
                1 => (b ^ c ^ d, 0x6ED9EBA1),
                2 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e]) {
            *x = x.wrapping_add(y);
        }
    }
    h.iter().map(|w| format!("{w:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digests_match_rfc_vectors() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(md5_hex(b"The quick brown fox jumps over the lazy dog"), "9e107d9d372bb6826bd81d3542a419d6");
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        // 跨两个分块的长度
        let long = vec![b'a'; 1000];
        assert_eq!(sha1_hex(&long), "291e9a6c66994949b57ba5e650361e98fc36b1ba");
        assert_eq!(md5_hex(&long), "cabe45dcc9ae5b66ba86600cca6b8ba8");
    }

    #[test]
    fn encodings_round_trip() {
        let bytes = "中文 abc".as_bytes();
        for enc in ["utf8", "utf-8", "base64", "hex", "binary", "latin1"] {
            let s = decode(bytes, enc).unwrap();
            assert_eq!(encode(&s, enc).unwrap(), bytes, "{enc}");
        }
        let s = decode(&encode("héllo", "ucs2").unwrap(), "utf-16le").unwrap();
        assert_eq!(s, "héllo");
        assert!(decode(b"x", "gbk").is_none());
        assert!(encode("zz", "hex").is_none());
    }
}
