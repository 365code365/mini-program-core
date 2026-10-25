//! 伪粗体对照图（text/embolden.rs）：同一段字，上一行是真实粗体字面（系统主字体），下一行是没有粗体字面时的伪粗体。
//! `cargo run --release --example faux_bold_compare -- <输出.png>`；调强度看 `text::faux_bold_strength`
use mini_render::text::TextRenderer;
use mini_render::{Canvas, Color, Paint};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "target/faux_bold.png".into());
    let real = TextRenderer::load_system_font().expect("系统字体");
    let faux = TextRenderer::from_file("assets/SourceHanSansSC-Regular.otf").expect("内置字体");
    assert!(!faux.has_bold_face(), "内置字体该没有粗体字面");
    let mut c = Canvas::new(900, 560);
    c.clear(Color::WHITE);
    let paint = Paint::new().with_color(Color::from_hex(0x1e2329));
    let mut y = 60.0;
    for size in [34.0f32, 28.0, 22.0] {
        real.draw_text_weighted(&mut c, "待缴提醒 我的户号 缴费小贴士", 20.0, y, size, 0.0, false, &paint);
        real.draw_text_weighted(&mut c, "真", 820.0, y, size, 0.0, false, &paint);
        y += size * 1.5;
        real.draw_text_weighted(&mut c, "待缴提醒 我的户号 缴费小贴士", 20.0, y, size, 0.0, true, &paint);
        y += size * 1.5;
        faux.draw_text_weighted(&mut c, "待缴提醒 我的户号 缴费小贴士", 20.0, y, size, 0.0, true, &paint);
        faux.draw_text_weighted(&mut c, "伪", 820.0, y, size, 0.0, false, &paint);
        y += size * 1.9;
    }
    let rgba: Vec<u8> = c.pixels().iter().flat_map(|p| [p.r, p.g, p.b, 255]).collect();
    image::save_buffer(&out, &rgba, 900, 560, image::ColorType::Rgba8).unwrap();
    println!("{out}");
}
