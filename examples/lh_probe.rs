//! 排查工具：打印候选字体的行度量明细（ascent/descent/line_gap/new_line_size）。
use fontdue::{Font, FontSettings};

fn main() {
    let candidates = [
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    ];
    for path in candidates {
        if !std::path::Path::new(path).exists() {
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else { continue };
        let Ok(font) = Font::from_bytes(bytes.as_slice(), FontSettings { scale: 40.0, ..Default::default() }) else {
            println!("{path}: 解析失败");
            continue;
        };
        if let Some(m) = font.horizontal_line_metrics(16.0) {
            println!(
                "{path}\n  16px: ascent={:.3} descent={:.3} line_gap={:.3} new_line={:.3} | a-d={:.3}({:.4}) newline_ratio={:.4}",
                m.ascent, m.descent, m.line_gap, m.new_line_size,
                m.ascent - m.descent, (m.ascent - m.descent) / 16.0,
                m.new_line_size / 16.0
            );
        }
    }
}
