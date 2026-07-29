//! 临时探针：量 fontdue 不同加载参数下的常驻内存
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
}
