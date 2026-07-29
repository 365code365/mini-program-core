//! 字体内存探针：量 fontdue 的**预展开**解析 vs 惰性解析的常驻内存差距。
//!
//! 背景见 `doc/引擎测试说明.md` 的「内存」一节：引擎的内存 98% 花在字体上，
//! 因为 fontdue 在 `Font::from_bytes` 时就把字体里**所有**字形的几何展开好
//! （CJK 字体 29352 个字形 → 约 300MB/字面）。这个探针用来给「换惰性后端」
//! 这件事提供实测依据。
//!
//! ```bash
//! cargo run --release --example font_mem_probe            # 默认 scale 40
//! cargo run --release --example font_mem_probe -- scale12 # 换 scale 看几何精度对内存的影响
//! ```
use fontdue::{Font, FontSettings};

fn rss_mb() -> f32 {
    let pid = std::process::id().to_string();
    let out = std::process::Command::new("ps").args(["-o", "rss=", "-p", &pid]).output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().parse::<f32>().map(|kb| kb / 1024.0).unwrap_or(0.0),
        Err(_) => 0.0,
    }
}

fn main() {
    let path = "/System/Library/Fonts/Hiragino Sans GB.ttc";
    let mode = std::env::args().nth(1).unwrap_or_else(|| "default".into());
    let base = rss_mb();
    let data = std::fs::read(path).unwrap();
    let after_read = rss_mb();
    let settings = match mode.as_str() {
        "nosub" => FontSettings { scale: 40.0, load_substitutions: false, ..Default::default() },
        "scale12" => FontSettings { scale: 12.0, ..Default::default() },
        "scale200" => FontSettings { scale: 200.0, ..Default::default() },
        _ => FontSettings { scale: 40.0, ..Default::default() },
    };
    let f = Font::from_bytes(data.as_slice(), settings).unwrap();
    let after_parse = rss_mb();
    drop(data);
    let after_drop = rss_mb();
    println!(
        "{mode:>9}: 起始 {base:.0}MB  读盘后 {after_read:.0}MB  解析后 {after_parse:.0}MB  丢掉字节后 {after_drop:.0}MB  字形数 {}",
        f.glyph_count()
    );
    std::hint::black_box(&f);

    // 对照：惰性解析（ttf-parser 零拷贝借用字节，不预展开字形几何）
    let data2 = std::fs::read(path).unwrap();
    let before = rss_mb();
    let face = ttf_parser::Face::parse(&data2, 0).unwrap();
    let after = rss_mb();
    println!(
        "{:>9}: 惰性解析(ttf-parser) 解析前 {before:.0}MB 解析后 {after:.0}MB  字形数 {}",
        "lazy",
        face.number_of_glyphs()
    );
    std::hint::black_box(&face);
}
