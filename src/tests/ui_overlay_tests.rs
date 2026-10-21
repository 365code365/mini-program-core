//! UI 覆盖层测试（Toast/Loading/Modal）
//! 测试文字渲染、图标绘制、布局计算等功能

use crate::{Canvas, Color, Paint};
use crate::text::TextRenderer;

/// 测试 TextRenderer 的 baseline 定位
#[test]
fn test_text_baseline_positioning() {
    let tr = TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf"))
        .expect("Failed to load font");
    
    let mut canvas = Canvas::new(200, 50);
    canvas.clear(Color::WHITE);
    
    let paint = Paint::new().with_color(Color::BLACK);
    let font_size = 16.0;
    
    // baseline 应该在字体高度的约 80% 位置
    let baseline_y = font_size * 0.85;
    tr.draw_text(&mut canvas, "Hello 你好", 10.0, baseline_y, font_size, &paint);
    
    // 检查文字是否渲染到了 canvas 上（应该有非白色像素）
    let pixels = canvas.pixels();
    let has_text = pixels.iter().any(|p| p.r != 255 || p.g != 255 || p.b != 255);
    assert!(has_text, "Text should be rendered on canvas");
}

/// 测试文字测量
#[test]
fn test_text_measurement() {
    let tr = TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf"))
        .expect("Failed to load font");
    
    let font_size = 14.0;
    
    // 测量英文
    let width_en = tr.measure_text("Hello", font_size);
    assert!(width_en > 0.0, "English text width should be positive");
    
    // 测量中文
    let width_cn = tr.measure_text("你好", font_size);
    assert!(width_cn > 0.0, "Chinese text width should be positive");
    
    // 中文字符通常比英文宽
    let width_per_cn_char = width_cn / 2.0;
    let width_per_en_char = width_en / 5.0;
    assert!(width_per_cn_char > width_per_en_char, "Chinese chars should be wider than English chars");
}

/// 测试文字高度测量
#[test]
fn test_text_height_measurement() {
    let tr = TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf"))
        .expect("Failed to load font");
    
    let font_size = 16.0;
    let height = tr.measure_height(font_size);
    
    // 高度应该接近字体大小
    assert!(height > font_size * 0.5, "Text height should be at least half of font size");
    assert!(height < font_size * 2.0, "Text height should be less than double font size");
}

/// 测试透明背景上的文字渲染
#[test]
fn test_text_on_transparent_background() {
    let tr = TextRenderer::from_bytes(include_bytes!("../../assets/ArialUnicode.ttf"))
        .expect("Failed to load font");
    
    let mut canvas = Canvas::new(100, 30);
    canvas.clear(Color::TRANSPARENT);
    
    let paint = Paint::new().with_color(Color::WHITE);
    let font_size = 14.0;
    let baseline_y = font_size * 0.85;
    
    tr.draw_text(&mut canvas, "Test", 5.0, baseline_y, font_size, &paint);
    
    // 检查是否有白色像素（文字）
    let pixels = canvas.pixels();
    let has_white = pixels.iter().any(|p| p.r == 255 && p.g == 255 && p.b == 255 && p.a > 0);
    assert!(has_white, "White text should be rendered on transparent background");
}

/// 测试 Canvas 像素操作
#[test]
fn test_canvas_pixel_operations() {
    let mut canvas = Canvas::new(10, 10);
    canvas.clear(Color::from_hex(0x4C4C4C));
    
    // 检查清除颜色
    let pixels = canvas.pixels();
    assert_eq!(pixels[0].r, 0x4C);
    assert_eq!(pixels[0].g, 0x4C);
    assert_eq!(pixels[0].b, 0x4C);
    
    // 设置单个像素
    canvas.set_pixel(5, 5, Color::RED);
    let pixels = canvas.pixels();
    let idx = 5 * 10 + 5;
    assert_eq!(pixels[idx].r, 255);
    assert_eq!(pixels[idx].g, 0);
    assert_eq!(pixels[idx].b, 0);
}

/// 测试 Color 混合
#[test]
fn test_color_blending() {
    // 半透明红色覆盖白色背景
    let src = Color::new(255, 0, 0, 128);
    let dst = Color::WHITE;
    let result = src.blend(&dst);
    
    // 结果应该是粉红色
    assert!(result.r > 200, "Red channel should be high");
    assert!(result.g > 100 && result.g < 200, "Green channel should be medium");
    assert!(result.b > 100 && result.b < 200, "Blue channel should be medium");
}

/// 测试完全透明不影响背景
#[test]
fn test_transparent_blend() {
    let src = Color::TRANSPARENT;
    let dst = Color::from_hex(0x123456);
    let result = src.blend(&dst);
    
    assert_eq!(result.r, dst.r);
    assert_eq!(result.g, dst.g);
    assert_eq!(result.b, dst.b);
}

/// 测试完全不透明覆盖背景
#[test]
fn test_opaque_blend() {
    let src = Color::RED;
    let dst = Color::BLUE;
    let result = src.blend(&dst);
    
    assert_eq!(result.r, 255);
    assert_eq!(result.g, 0);
    assert_eq!(result.b, 0);
}

// 提示框 / Modal 的布局与转圈：真正的算式在 host::ui_overlay::geometry / icons，
// 用例跟着放在那两个模块里（从前这里是照抄一份旧算式自己验自己，与实现脱节）。

/// 测试圆角矩形边界检查
#[test]
fn test_rounded_rect_bounds() {
    let x = 100i32;
    let y = 100i32;
    let w = 200i32;
    let h = 150i32;
    let radius = 12i32;
    
    // 检查角落点是否在圆角内
    let check_corner = |px: i32, py: i32| -> bool {
        let in_corner = (px < x + radius || px >= x + w - radius) &&
                       (py < y + radius || py >= y + h - radius);
        if in_corner {
            let cx = if px < x + radius { x + radius } else { x + w - radius };
            let cy = if py < y + radius { y + radius } else { y + h - radius };
            let dist = (((px - cx) * (px - cx) + (py - cy) * (py - cy)) as f32).sqrt();
            dist <= radius as f32
        } else {
            true
        }
    };
    
    // 左上角外部点应该不在矩形内
    assert!(!check_corner(x, y));
    // 中心点应该在矩形内
    assert!(check_corner(x + w/2, y + h/2));
    // 边缘中点应该在矩形内
    assert!(check_corner(x + w/2, y));
}


/// 测试渐变 fallback 解析
#[test]
fn test_gradient_fallback_parsing() {
    use crate::parser::WxssParser;
    
    // 测试 linear-gradient 解析
    let css = r#"
        .test1 { background: linear-gradient(135deg, #ff6b35 0%, #ff8f5a 100%); }
        .test2 { background: linear-gradient(180deg, #fff5f0 0%, #f5f5f5 200rpx); }
        .test3 { background-color: #ff4d4f; }
    "#;
    
    let mut parser = WxssParser::new(css);
    let stylesheet = parser.parse().expect("Failed to parse CSS");
    
    // test1 应该提取第一个颜色 #ff6b35
    let styles1 = stylesheet.get_styles(&["test1"], "view");
    assert!(styles1.contains_key("background"), "test1 should have background");
    
    // test2 应该提取第一个颜色 #fff5f0
    let styles2 = stylesheet.get_styles(&["test2"], "view");
    assert!(styles2.contains_key("background"), "test2 should have background");
    
    // test3 应该是纯色 #ff4d4f
    let styles3 = stylesheet.get_styles(&["test3"], "view");
    assert!(styles3.contains_key("background-color"), "test3 should have background-color");
}
