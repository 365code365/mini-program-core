//! `FileSystemManager.unzip`：读 zip 的中央目录，逐项解到目标目录。
//!
//! 只认两种压缩方式：0（存储）与 8（deflate）—— 这两种覆盖了绝大多数打包工具的产物；
//! 别的方式（bzip2 / lzma / zstd）与加密条目明确报错，不解出半个目录。
//! 不支持 zip64（单包 4GB 以上，小程序本地文件上限远低于此）。

use std::io::Read;
use std::path::{Component, Path};

fn u16le(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(at)?, *b.get(at + 1)?]))
}

fn u32le(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([*b.get(at)?, *b.get(at + 1)?, *b.get(at + 2)?, *b.get(at + 3)?]))
}

/// 一个条目：(包内路径, 解压后的内容)；目录条目的内容为 None
pub fn entries(data: &[u8]) -> Result<Vec<(String, Option<Vec<u8>>)>, String> {
    // 末尾的「中央目录结束记录」：签名 0x06054b50，后面最多跟 65535 字节注释
    let min = data.len().saturating_sub(22 + 65535);
    let eocd = (min..data.len().saturating_sub(21))
        .rev()
        .find(|&i| u32le(data, i) == Some(0x0605_4b50))
        .ok_or("not a zip file")?;
    let count = u16le(data, eocd + 10).ok_or("bad zip")? as usize;
    let mut at = u32le(data, eocd + 16).ok_or("bad zip")? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if u32le(data, at) != Some(0x0201_4b50) {
            return Err("bad central directory".into());
        }
        let flags = u16le(data, at + 8).ok_or("bad zip")?;
        let method = u16le(data, at + 10).ok_or("bad zip")?;
        let csize = u32le(data, at + 20).ok_or("bad zip")? as usize;
        let usize_ = u32le(data, at + 24).ok_or("bad zip")? as usize;
        let nlen = u16le(data, at + 28).ok_or("bad zip")? as usize;
        let xlen = u16le(data, at + 30).ok_or("bad zip")? as usize;
        let clen = u16le(data, at + 32).ok_or("bad zip")? as usize;
        let local = u32le(data, at + 42).ok_or("bad zip")? as usize;
        let name_bytes = data.get(at + 46..at + 46 + nlen).ok_or("bad zip")?;
        let name = String::from_utf8_lossy(name_bytes).replace('\\', "/");
        at += 46 + nlen + xlen + clen;
        if name.ends_with('/') {
            out.push((name, None));
            continue;
        }
        if flags & 1 != 0 {
            return Err(format!("encrypted entry not supported: {name}"));
        }
        // 本地文件头的文件名 / 扩展区长度可能与中央目录不同，按本地头算数据起点
        if u32le(data, local) != Some(0x0403_4b50) {
            return Err("bad local header".into());
        }
        let lnlen = u16le(data, local + 26).ok_or("bad zip")? as usize;
        let lxlen = u16le(data, local + 28).ok_or("bad zip")? as usize;
        let start = local + 30 + lnlen + lxlen;
        let raw = data.get(start..start + csize).ok_or("truncated zip")?;
        let content = match method {
            0 => raw.to_vec(),
            8 => {
                let mut v = Vec::with_capacity(usize_);
                flate2::read::DeflateDecoder::new(raw).read_to_end(&mut v).map_err(|e| e.to_string())?;
                v
            }
            m => return Err(format!("compression method {m} not supported: {name}")),
        };
        out.push((name, Some(content)));
    }
    Ok(out)
}

/// 条目路径不能跳出目标目录（`../x`、绝对路径）
pub fn safe_rel(name: &str) -> bool {
    let p = Path::new(name);
    !name.is_empty() && p.components().all(|c| matches!(c, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 手搓一个只含存储条目的 zip（不依赖外部工具）
    fn stored_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, body) in files {
            let off = out.len() as u32;
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend_from_slice(&0u32.to_le_bytes()); // crc（解压不校验）
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(body);
            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&(body.len() as u32).to_le_bytes());
            central.extend_from_slice(&(body.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name.len() as u16).to_le_bytes());
            central.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            central.extend_from_slice(&off.to_le_bytes());
            central.extend_from_slice(name.as_bytes());
        }
        let cd_off = out.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(central.len() as u32).to_le_bytes());
        out.extend_from_slice(&cd_off.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    #[test]
    fn reads_stored_entries() {
        let z = stored_zip(&[("a.txt", b"hello"), ("d/", b""), ("d/b.txt", b"x")]);
        let e = entries(&z).unwrap();
        assert_eq!(e.len(), 3);
        assert_eq!(e[0], ("a.txt".to_string(), Some(b"hello".to_vec())));
        assert_eq!(e[1].1, None);
        assert_eq!(e[2].1.as_deref(), Some(&b"x"[..]));
        assert!(entries(b"not a zip").is_err());
    }

    #[test]
    fn rejects_escaping_names() {
        assert!(safe_rel("a/b.txt"));
        assert!(!safe_rel("../x"));
        assert!(!safe_rel("/etc/passwd"));
    }
}
