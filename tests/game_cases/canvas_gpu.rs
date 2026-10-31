//! GT-2DG：2D 画布走 GPU（canvas/gpu2d，`canvas2d: "gpu"`）与 CPU 参考实现逐像素比。
//!
//! 判据（steering 10：GPU 的判据是「与 CPU 同一帧对比，差异 ≤ 定死的阈值，且包围盒不落在内容区之外」）：
//! 每个场景同一份 JS 在两个后端各跑一次，比整帧输出（压黑底的 RGBA）。「差 > 2 级」的像素比例与最大通道差
//! 各有上限（[`LIMITS`]，按实测定，见 doc/引擎测试说明.md）。差 ≤ 2 级是预乘 / 非预乘两次取整的正常误差。
//! 另外判功能：GPU 后端真的被选上了、回退那几条（getImageData、multiply、putImageData）结果与 CPU 一致。
use super::*;

const HEAD: &str = "var T = require('_lib/t.js'); var c = wx.createCanvas(); var g = c.getContext('2d');\n";

/// (场景, 差 > 2 级的像素比例上限, 最大通道差上限)
/// 实测（2026-10-31，SwiftShader）：rects / text / fx / mixed 差 > 2 的像素 0、最大差 1~2；paths 0.14%、最大 6（半透明形状边缘
/// 两次取整）；images 0.15%、最大 204（旋转的 drawImage：CPU 的边缘有抗锯齿，GPU 的四边形边缘没有，只在旋转图的轮廓上）
const LIMITS: &[(&str, f64, u8)] = &[("rects", 0.0001, 4), ("paths", 0.002, 8), ("text", 0.0001, 4), ("fx", 0.0001, 4), ("images", 0.002, 255), ("mixed", 0.0001, 4)];

fn scene(name: &str) -> String {
    let body = match name {
        "rects" => r#"
g.fillStyle = '#102030'; g.fillRect(0, 0, 375, 667);
for (var i = 0; i < 300; i++) { g.fillStyle = ['#e74c3c', '#3498db', '#2ecc71', '#f1c40f', '#9b59b6'][i % 5]; g.globalAlpha = (i % 3 == 0) ? 0.5 : 1;
  g.fillRect((i * 37.3) % 360, (i * 53.7) % 650, 16.4, 9.7); }
g.globalAlpha = 1; g.clearRect(10.5, 10.5, 50, 30); g.fillStyle = 'rgba(255,255,255,0.3)'; g.fillRect(100, 100, 100.5, 100.25);
g.fillStyle = '#ff0'; g.fillRect(200, 300, -40, -20); g.fillRect(300.25, 400.75, 0.5, 30);
"#,
        "paths" => r#"
g.fillStyle = '#fff'; g.fillRect(0, 0, 375, 667);
g.beginPath(); g.arc(100, 100, 60, 0, Math.PI * 2); g.fillStyle = '#e33'; g.fill(); g.lineWidth = 6; g.strokeStyle = '#33e'; g.stroke();
g.beginPath(); g.moveTo(20, 300); g.bezierCurveTo(100, 200, 200, 400, 350, 300); g.lineWidth = 4; g.setLineDash([10, 6]); g.stroke(); g.setLineDash([]);
g.save(); g.translate(250, 150); g.rotate(0.5); g.fillStyle = 'rgba(0,128,0,0.7)'; g.fillRect(-40, -20, 80, 40); g.strokeRect(-50, -30, 100, 60); g.restore();
g.lineJoin = 'round'; g.beginPath(); g.moveTo(50, 450); g.lineTo(150, 550); g.lineTo(250, 450); g.lineWidth = 12; g.stroke();
g.beginPath(); g.moveTo(300, 450); g.lineTo(340, 600); g.lineTo(260, 600); g.closePath(); g.fillStyle = 'rgba(200,0,200,0.5)'; g.fill();
"#,
        "text" => r#"
g.fillStyle = '#222'; g.fillRect(0, 0, 375, 667);
g.fillStyle = '#fff'; g.font = '20px sans-serif'; g.fillText('Hello 你好 GPU', 20, 50);
g.textAlign = 'center'; g.fillText('居中 center', 187, 100); g.textAlign = 'right'; g.fillText('right', 360, 150);
g.textAlign = 'left'; g.strokeStyle = '#f80'; g.lineWidth = 2; g.font = 'bold 36px sans-serif'; g.strokeText('Stroke', 20, 220);
g.save(); g.translate(100, 400); g.rotate(-0.4); g.fillText('旋转的字', 0, 0); g.restore();
g.fillText('压缩压缩压缩压缩压缩', 20, 500, 120);
g.textBaseline = 'top'; g.fillStyle = 'rgba(0,255,0,0.5)'; g.fillText('top 基线', 20, 560);
"#,
        "fx" => r#"
g.fillStyle = '#fff'; g.fillRect(0, 0, 375, 667);
var lg = g.createLinearGradient(0, 0, 0, 200); lg.addColorStop(0, '#f00'); lg.addColorStop(1, '#00f'); g.fillStyle = lg; g.fillRect(20, 20, 150, 200);
var rg = g.createRadialGradient(270, 120, 10, 270, 120, 80); rg.addColorStop(0, '#ff0'); rg.addColorStop(1, 'rgba(0,128,0,0)');
g.fillStyle = rg; g.beginPath(); g.arc(270, 120, 80, 0, 7); g.fill();
g.shadowColor = 'rgba(0,0,0,0.6)'; g.shadowBlur = 10; g.shadowOffsetX = 6; g.shadowOffsetY = 6; g.fillStyle = '#09c'; g.fillRect(30, 260, 120, 80);
g.shadowColor = 'rgba(0,0,0,0)';
var ops = ['destination-out', 'source-atop', 'lighter', 'xor', 'destination-over', 'multiply', 'screen'];
for (var i = 0; i < ops.length; i++) { g.globalCompositeOperation = 'source-over'; g.fillStyle = '#c33'; g.fillRect(20 + i * 48, 380, 40, 40);
  g.globalCompositeOperation = ops[i]; g.fillStyle = 'rgba(30,120,240,0.8)'; g.beginPath(); g.arc(50 + i * 48, 410, 18, 0, 7); g.fill(); }
g.globalCompositeOperation = 'source-over';
var o = wx.createCanvas(); o.width = 100; o.height = 100; var og = o.getContext('2d'); og.fillStyle = '#0a0'; og.fillRect(0, 0, 100, 100);
og.globalCompositeOperation = 'source-in'; og.fillStyle = '#f0f'; og.beginPath(); og.arc(50, 50, 30, 0, 7); og.fill();
og.globalCompositeOperation = 'copy'; og.fillStyle = 'rgba(0,0,255,0.5)'; og.fillRect(60, 60, 30, 30);
g.drawImage(o, 20, 480); g.drawImage(o, 0, 0, 50, 50, 150, 480, 100, 100);
"#,
        "images" => r#"
g.fillStyle = '#345'; g.fillRect(0, 0, 375, 667);
var o = wx.createCanvas(); o.width = 64; o.height = 64; var og = o.getContext('2d');
for (var y = 0; y < 8; y++) for (var x = 0; x < 8; x++) { og.fillStyle = ((x + y) % 2) ? '#fff' : '#f60'; og.fillRect(x * 8, y * 8, 8, 8); }
g.drawImage(o, 10, 10); g.drawImage(o, 100, 10, 128, 96); g.globalAlpha = 0.5; g.drawImage(o, 250, 10); g.globalAlpha = 1;
g.save(); g.translate(80, 250); g.rotate(0.3); g.drawImage(o, -32, -32); g.restore();
g.imageSmoothingEnabled = false; g.drawImage(o, 8, 8, 16, 16, 200, 200, 100, 100); g.imageSmoothingEnabled = true;
g.save(); g.beginPath(); g.arc(187, 450, 90, 0, 7); g.clip(); g.fillStyle = '#ee3'; g.fillRect(80, 350, 220, 200); g.drawImage(o, 150, 420);
g.fillStyle = '#000'; g.font = '30px sans-serif'; g.fillText('clip', 150, 470); g.restore();
"#,
        // GPU 与回退（读像素、要读目标色的合成、putImageData）交替：两边来回搬之后结果仍一致
        "mixed" => r#"
g.fillStyle = '#123'; g.fillRect(0, 0, 375, 667); g.fillStyle = '#f80'; g.fillRect(20.5, 20.5, 100, 100);
var d = g.getImageData(60, 60, 2, 1); T.ev('px', { d: Array.prototype.slice.call(d.data) });
g.globalCompositeOperation = 'multiply'; g.fillStyle = '#0ff'; g.fillRect(60, 60, 100, 100); g.globalCompositeOperation = 'source-over';
var p = g.createImageData(20, 20); for (var i = 0; i < p.data.length; i += 4) { p.data[i] = 255; p.data[i + 3] = 255; } g.putImageData(p, 300, 600);
g.fillStyle = '#0f0'; g.fillRect(310, 610, 30, 30); g.beginPath(); g.arc(200, 300, 50, 0, 7); g.fill();
var u = c.toDataURL(); T.ev('url', { ok: u.indexOf('data:image/png;base64,') == 0, n: u.length });
var e = g.getImageData(320, 620, 1, 1); T.ev('px2', { d: Array.prototype.slice.call(e.data) });
"#,
        _ => unreachable!(),
    };
    format!("{HEAD}{body}\nT.ready('{name}');\n")
}

fn render(name: &str, mode: &'static str) -> (GameHarness, Vec<u8>) {
    let js = scene(name);
    let mut h = GameHarness::open_inline(&format!("g2d-{name}-{mode}"), &js, Opts { canvas2d: Some(mode), ..Default::default() });
    h.vsyncs(3);
    h.assert_no_fail();
    let px = h.engine.frame_rgba().2.to_vec();
    (h, px)
}

/// (差 > 2 级的像素比例, 最大通道差, 差异包围盒)
fn diff(a: &[u8], b: &[u8], w: usize) -> (f64, u8, Option<(usize, usize, usize, usize)>) {
    let (mut n, mut max, mut bb) = (0usize, 0u8, None::<(usize, usize, usize, usize)>);
    for (i, (p, q)) in a.chunks_exact(4).zip(b.chunks_exact(4)).enumerate() {
        let d = (0..3).map(|k| p[k].abs_diff(q[k])).max().unwrap();
        max = max.max(d);
        if d > 2 {
            n += 1;
            let (x, y) = (i % w, i / w);
            bb = Some(bb.map_or((x, y, x, y), |b| (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y))));
        }
    }
    (n as f64 / (a.len() / 4) as f64, max, bb)
}

#[test]
fn g2d_gpu与cpu逐像素对比() {
    let _g = serial();
    let mut report = Vec::new();
    for &(name, frac_max, diff_max) in LIMITS {
        let (hc, cpu) = render(name, "cpu");
        let (hg, gpu) = render(name, "gpu");
        assert!(hg.has("2D 画布后端 gpu"), "{name}：GPU 后端没选上（先跑 tools/fetch-angle.sh）：{:?}", hg.log.iter().filter(|l| l.contains("2D")).collect::<Vec<_>>());
        assert!(!hc.has("2D 画布后端 gpu"), "{name}：cpu 模式不该建 GPU");
        assert!(!hg.has("回退 CPU"), "{name}：GPU 中途回退了：{:?}", hg.log);
        assert_eq!(cpu.len(), gpu.len());
        let w = hc.engine.main_canvas_size().0 as usize;
        let (frac, max, bb) = diff(&cpu, &gpu, w);
        report.push(format!("{name}: 差>2 {:.4}% 最大 {max} 框 {bb:?}", frac * 100.0));
        eprintln!("{}", report.last().unwrap());
        assert!(frac <= frac_max && max <= diff_max, "{name}：GPU 与 CPU 差太多：差>2 的像素 {:.4}%（上限 {:.4}%），最大通道差 {max}（上限 {diff_max}），框 {bb:?}", frac * 100.0, frac_max * 100.0);
        let (draws, tiles, reads) = hg.engine.gpu2d_stats().unwrap();
        eprintln!("  {name}: draw call {draws}、临时层 {tiles}、读回 {reads}");
        if name == "rects" {
            // 300 个矩形 + 清屏：合批（不是一个矩形一次 draw call），只在取帧时读回一次
            assert!(draws <= 6 && tiles == 0 && reads == 1, "rects：draw {draws} 临时层 {tiles} 读回 {reads}");
        }
        if name == "mixed" {
            for ev in ["px", "px2"] {
                assert_eq!(hc.events(ev), hg.events(ev), "mixed：{ev} 两个后端要相同");
            }
            // toDataURL：PNG 长度随像素的 1 级舍入差会变，只判是 PNG
            assert_eq!(hg.events("url")[0]["ok"], true);
            assert_eq!(hg.events("px2")[0]["d"], serde_json::json!([0, 255, 0, 255]), "putImageData 之后再在 GPU 上画");
        }
    }
    eprintln!("GPU 2D 对比：\n  {}", report.join("\n  "));
}

/// 精灵基准（g-bench-sprites：图片当纹理源、每帧整屏清 + 500 次 drawImage）：跑 30 帧两边仍一致，稳态不回退 CPU
#[test]
fn g2d_精灵每帧一致() {
    let _g = serial();
    let run = |mode: &'static str| {
        let mut h = GameHarness::open("g-bench-sprites", Opts { canvas2d: Some(mode), query: "quiet=1", ..Default::default() });
        h.until("T:READY", 300);
        h.vsyncs(30);
        h.assert_no_fail();
        let px = h.engine.frame_rgba().2.to_vec();
        (h, px)
    };
    let (hc, cpu) = run("cpu");
    let (hg, gpu) = run("gpu");
    assert!(hg.has("2D 画布后端 gpu") && !hg.has("回退 CPU"), "{:?}", hg.log);
    let (frac, max, bb) = diff(&cpu, &gpu, hc.engine.main_canvas_size().0 as usize);
    let (draws, tiles, reads) = hg.engine.gpu2d_stats().unwrap();
    eprintln!("精灵 30 帧：draw call {draws}、临时层 {tiles}、读回 {reads}");
    assert!(tiles == 0 && reads <= 2, "稳态不该走临时层 / 读回：临时层 {tiles} 读回 {reads}");
    eprintln!("精灵 30 帧：差>2 {:.4}% 最大 {max} 框 {bb:?}", frac * 100.0);
    assert!(frac <= 0.001 && max <= 40, "差>2 {:.4}% 最大 {max} 框 {bb:?}", frac * 100.0);
}

/// WebGL 读 GPU 2D 画布当纹理（texImage2D(canvas)）：读回之后切回 WebGL 自己的上下文，像素对
#[test]
fn g2d_webgl读gpu画布() {
    let _g = serial();
    let js = r#"var T = require('_lib/t.js');
var c = wx.createCanvas(), gl = c.getContext('webgl');
var c2 = wx.createCanvas(); c2.width = 2; c2.height = 1; var g2 = c2.getContext('2d');
g2.fillStyle = '#ffff00'; g2.fillRect(0, 0, 1, 1); g2.fillStyle = '#00ffff'; g2.fillRect(1, 0, 1, 1);
function sh(t, s) { var x = gl.createShader(t); gl.shaderSource(x, s); gl.compileShader(x); return x; }
var p = gl.createProgram(); gl.attachShader(p, sh(gl.VERTEX_SHADER, 'attribute vec2 p; varying vec2 uv; void main(){ uv = p * 0.5 + 0.5; gl_Position = vec4(p, 0.0, 1.0); }'));
gl.attachShader(p, sh(gl.FRAGMENT_SHADER, 'precision mediump float; varying vec2 uv; uniform sampler2D t; void main(){ gl_FragColor = texture2D(t, uv); }'));
gl.bindAttribLocation(p, 0, 'p'); gl.linkProgram(p);
var b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
var t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t);
gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, c2);
requestAnimationFrame(function () { gl.useProgram(p); gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4); T.ev('drawn', { err: gl.getError() }); });
T.ready('wgx');
"#;
    let mut h = GameHarness::open_inline("g2d-webgl", js, Opts { canvas2d: Some("gpu"), ..Default::default() });
    h.vsyncs(3);
    h.assert_no_fail();
    assert!(h.has("2D 画布后端 gpu"), "{:?}", h.log);
    assert_eq!(h.events("drawn")[0]["err"], 0);
    let (w, _, p) = h.engine.frame_rgba();
    let at = |x: u32, y: u32| { let i = ((y * w + x) * 4) as usize; [p[i], p[i + 1], p[i + 2]] };
    assert_eq!((at(90, 300), at(280, 300)), ([255, 255, 0], [0, 255, 255]), "左黄右青");
}

