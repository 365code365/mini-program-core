//! vendor/fontdue（字形惰性展开、cmap 按需查、轮廓线段封顶缓存）与上游 fontdue 0.9.3 逐位一致：
//! 同一份字体、同一组设置，每个字符的字形编号、每个字形在几个字号下的度量与光栅化位图都相同。
//!
//! - Arial / Monaco：整个字体的每个字形都比（几千个，上游预展开也就几 MB）；
//! - SourceHanSans（6.5 万字形，上游预展开要 400MB）：只比 cmap（全部 Unicode 码位）与常用汉字那一段的光栅化，
//!   并且光栅化走两遍：第二遍时轮廓线段多半已被封顶缓存丢掉、要重新展开，结果也必须一样。
use fontdue::{Font, FontSettings};
use fontdue_upstream::{Font as Upstream, FontSettings as UpSettings};
use std::sync::Arc;

const SIZES: [f32; 4] = [11.0, 16.0, 28.5, 48.0];

fn load(path: &str) -> (Font, Vec<u8>) {
    let bytes = std::fs::read(path).unwrap();
    let f = Font::from_shared(Arc::new(bytes.clone()), FontSettings { scale: 40.0, ..Default::default() }).unwrap();
    (f, bytes)
}

fn upstream(bytes: &[u8]) -> Upstream {
    Upstream::from_bytes(bytes, UpSettings { scale: 40.0, ..Default::default() }).unwrap()
}

/// 全部 Unicode 码位：惰性查 cmap 与上游建好的整张表一致
fn same_cmap(lazy: &Font, up: &Upstream, what: &str) {
    for c in (0u32..=0x10FFFF).filter_map(char::from_u32) {
        let want = up.chars().get(&c).map(|g| g.get()).unwrap_or(0);
        assert_eq!(lazy.lookup_glyph_index(c), want, "{what}：U+{:04X} 字形编号不同", c as u32);
    }
}

fn same_glyph(lazy: &Font, up: &Upstream, index: u16, what: &str) {
    for px in SIZES {
        let (ml, bl) = lazy.rasterize_indexed(index, px);
        let (mu, bu) = up.rasterize_indexed(index, px);
        assert_eq!(format!("{ml:?}"), format!("{mu:?}"), "{what}：字形 {index} @{px} 度量不同");
        assert!(bl == bu, "{what}：字形 {index} @{px} 位图不同");
        assert_eq!(format!("{:?}", lazy.metrics_indexed(index, px)), format!("{:?}", up.metrics_indexed(index, px)));
    }
}

#[test]
fn 小字体_每个字形都与上游逐位一致() {
    for path in ["assets/Arial.ttf", "assets/Monaco.ttf"] {
        let (lazy, bytes) = load(path);
        let up = upstream(&bytes);
        assert_eq!(lazy.glyph_count(), up.glyph_count());
        same_cmap(&lazy, &up, path);
        for i in 0..up.glyph_count() {
            same_glyph(&lazy, &up, i, path);
        }
        // 第二遍：轮廓线段早被封顶缓存丢了一大半，重新展开出来的必须一样
        for i in (0..up.glyph_count()).step_by(7) {
            same_glyph(&lazy, &up, i, path);
        }
        assert!(lazy.loaded_glyph_count() <= up.glyph_count() as usize);
    }
}

#[test]
fn 思源黑体_cmap_全码位一致_常用汉字光栅化一致() {
    let path = "assets/SourceHanSansSC-Regular.otf";
    let (lazy, bytes) = load(path);
    let up = upstream(&bytes);
    same_cmap(&lazy, &up, path);
    let chars: Vec<char> = ('\u{4E00}'..='\u{4FFF}').chain('A'..='z').chain("，。！？、（）《》".chars()).collect();
    for pass in 0..2 {
        for &c in &chars {
            let i = up.lookup_glyph_index(c);
            same_glyph(&lazy, &up, i, &format!("{path} 第 {pass} 遍 {c}"));
        }
    }
    // 几何封顶：几千个字过一遍，常驻的轮廓线段仍在 1MB 上下（不是上游的整个字面）
    let (geometry, _) = lazy.heap_bytes();
    assert!(geometry < 3 << 20, "轮廓线段没有封顶：{geometry} 字节");
}
