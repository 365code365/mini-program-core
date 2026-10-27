//! GT-F 字体与文字（game-doc/05 §3）：loadFont、font 读回、measureText、getTextLineHeight、基线与对齐的像素。
use super::*;

const JS: &str = r#"
var T = require('_lib/t.js');
var c = wx.createCanvas(); var g = c.getContext('2d');
var r = {};
r.mono = wx.loadFont('fonts/m.ttf');
r.missing = wx.loadFont('fonts/none.ttf');
r.woff = wx.loadFont('fonts/fake.woff');
r.escape = wx.loadFont('../../x.ttf');
r.def = g.font;
g.font = 'bold 20px Arial'; r.bold = g.font;
g.font = 'nonsense'; r.kept = g.font;
g.save(); g.font = '12px serif'; g.restore(); r.restored = g.font;
g.font = '20px Arial';
var m = g.measureText('Hello');
r.m = m;
g.font = 'bold 20px Arial'; r.mb = g.measureText('Hello').width;
g.font = '20px ' + r.mono; r.mm = g.measureText('iiii').width; r.mw = g.measureText('WWWW').width;
r.lh = wx.getTextLineHeight({ fontSize: 20, fontFamily: 'Arial', text: '中文' });
r.lh2 = wx.getTextLineHeight({ fontSize: 20, fontFamily: 'Arial', text: 'abc' });
// 基线：top 画在 y=100 以下，bottom 画在 y=200 以上；对齐：right 的墨迹在 x=300 左边
g.fillStyle = '#fff'; g.fillRect(0, 0, c.width, c.height);
g.fillStyle = '#000'; g.font = '30px Arial';
g.textBaseline = 'top'; g.fillText('Hg', 10, 100);
g.textBaseline = 'bottom'; g.fillText('Hg', 10, 200);
g.textBaseline = 'alphabetic'; g.textAlign = 'right'; g.fillText('Hg', 300, 300);
g.textAlign = 'center'; g.fillText('Hg', 200, 400, 10);
T.ev('r', r);
T.done({});
"#;

fn ink(h: &mut GameHarness, x0: u32, y0: u32, x1: u32, y1: u32) -> Option<(u32, u32, u32, u32)> {
    let (w, _, px) = h.engine.frame_rgba();
    let mut b: Option<(u32, u32, u32, u32)> = None;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * w + x) * 4) as usize;
            if px[i] < 128 {
                b = Some(match b {
                    None => (x, y, x, y),
                    Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x), e.max(y)),
                });
            }
        }
    }
    b
}

#[test]
fn f1_f5_字体与文字() {
    let _g = serial();
    let mut h = GameHarness::open_inline("text", "", Opts::default());
    let fonts = h.dir.join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/Monaco.ttf"), fonts.join("m.ttf")).unwrap();
    std::fs::write(fonts.join("fake.woff"), b"wOFFxxxxxxxx").unwrap();
    h.engine.eval(&format!("(function(){{ var module = {{}}; {JS} }})()")).unwrap();
    h.vsyncs(1);
    h.assert_no_fail();
    let r = h.events("r").pop().expect("没有结果");
    // F1
    assert_eq!(r["mono"], "Monaco", "loadFont 返回 name 表的字族名");
    for k in ["missing", "woff", "escape"] {
        assert!(r[k].is_null(), "{k} 应当返回 null：{}", r[k]);
    }
    // F2：读回与非法值
    assert_eq!(r["def"], "10px sans-serif");
    assert_eq!(r["bold"], "bold 20px Arial");
    assert_eq!(r["kept"], "bold 20px Arial", "解析不了保留旧值");
    assert_eq!(r["restored"], "bold 20px Arial", "restore 还原 font");
    // F4：Chrome 上 20px Arial 的 "Hello" 是 45.57px（H 14.44 + e 11.12 + l 4.44 ×2 + o 11.12）
    let m = &r["m"];
    let w = m["width"].as_f64().unwrap();
    assert!((w - 45.57).abs() / 45.57 <= 0.02, "measureText 宽度 {w} 与 Chrome 的 45.57 差超过 2%");
    for k in ["actualBoundingBoxLeft", "actualBoundingBoxRight", "actualBoundingBoxAscent", "actualBoundingBoxDescent", "fontBoundingBoxAscent", "fontBoundingBoxDescent"] {
        assert!(m[k].is_number(), "TextMetrics 缺 {k}：{m}");
    }
    assert!(m["actualBoundingBoxAscent"].as_f64().unwrap() > 10.0 && m["fontBoundingBoxAscent"].as_f64().unwrap() > m["actualBoundingBoxAscent"].as_f64().unwrap() - 1.0);
    assert!(r["mb"].as_f64().unwrap() >= w, "粗体不比常规窄");
    assert!((r["mm"].as_f64().unwrap() - r["mw"].as_f64().unwrap()).abs() < 0.01, "loadFont 的等宽字体生效：iiii 与 WWWW 一样宽");
    // F5
    let (lh, lh2) = (r["lh"].as_f64().unwrap(), r["lh2"].as_f64().unwrap());
    assert!(lh >= 20.0 && lh2 >= 20.0 && lh >= lh2, "行高：中文 {lh} ≥ 西文 {lh2} ≥ 字号");
    // 基线与对齐的像素
    let top = ink(&mut h, 0, 60, 120, 160).expect("top 基线的字没画出来");
    // top = em 框顶（ascent:descent 缩到 1em，与 Chrome 一致）：Arial 0.905 / 0.212 → 30px 下 em 顶在基线上 24.3px，
    // H 字高 0.716em = 21.5px → H 顶在 y = 100 + 24.3 − 21.5 ≈ 102.8
    assert!(top.1 >= 102 && top.1 <= 104, "textBaseline=top：H 的墨迹顶约在 y=102.8，实际 {top:?}");
    let bottom = ink(&mut h, 0, 150, 120, 240).expect("bottom 的字没画出来");
    assert!(bottom.3 <= 200 && bottom.3 >= 194, "textBaseline=bottom：墨迹底在 y=200 以上，实际 {bottom:?}");
    let right = ink(&mut h, 200, 260, 375, 320).expect("right 对齐的字没画出来");
    assert!(right.2 <= 300 && right.2 >= 295, "textAlign=right：墨迹右缘在 x=300，实际 {right:?}");
    let narrow = ink(&mut h, 150, 360, 250, 420).expect("maxWidth 的字没画出来");
    assert!(narrow.2 - narrow.0 <= 11, "maxWidth=10：墨迹宽 ≤ 10，实际 {narrow:?}");
}
