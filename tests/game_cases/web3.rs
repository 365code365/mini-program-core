//! 高性能+（游戏 JS 在页面里、画布 / WebGL 交回渲染端回放）的 e2e：页面在 headless Chrome，渲染端是 devserver 里的
//! `GameEngine`（remote:true，SwiftShader）。像素从渲染端的输出帧取 —— 页面自己是透明的，只画覆盖层。
use super::*;
use game_harness::web::{WebHarness, WebOpts};

macro_rules! open {
    ($e:expr) => {
        match $e {
            Some(h) => h,
            None => return,
        }
    };
}

fn plus() -> WebOpts {
    WebOpts { plus: true, ..Default::default() }
}

/// 2D：上屏画布、矩形 / 路径 / 文字、measureText 缓存、getImageData 同步往返、页面里没有 DOM 画布
#[test]
fn plus_2d_画布与同步查询() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
var c = wx.createCanvas(), g = c.getContext('2d');
g.fillStyle = '#ff0000'; g.fillRect(0, 0, 100, 100);
g.fillStyle = '#0000ff'; g.beginPath(); g.arc(200, 200, 40, 0, Math.PI * 2); g.fill();
g.font = 'bold 20px sans-serif';
var m1 = g.measureText('hello'), m2 = g.measureText('hello');
g.fillStyle = '#ffffff'; g.fillText('hello', 20, 300);
var px = g.getImageData(10, 10, 2, 2).data;
var off = wx.createCanvas(); off.width = 4; off.height = 4;
T.ev('2d', { w: c.width, h: c.height, mw: m1.width, same: m1.width === m2.width, px: [px[0], px[1], px[2], px[3]], font: g.font,
  offW: off.width, dom: document.querySelectorAll('canvas').length, saved: g.save() });
"#;
    let mut h = open!(WebHarness::open_inline("plus2d", "{}", js, &[], plus()));
    h.until("T:EV 2d", 10.0).sleep(300);
    let e = &h.events("2d")[0];
    assert_eq!((e["w"].as_u64(), e["h"].as_u64(), e["offW"].as_u64(), e["dom"].as_u64()), (Some(375), Some(667), Some(4), Some(0)), "{e}");
    assert!(e["mw"].as_f64().unwrap() > 30.0 && e["same"] == true, "{e}");
    assert_eq!(e["px"], serde_json::json!([255, 0, 0, 255]), "getImageData 同步拿到渲染端的像素");
    assert_eq!(e["font"], "bold 20px sans-serif");
    assert_eq!(h.native_pixel(50, 50), [255, 0, 0]);
    assert_eq!(h.native_pixel(200, 200), [0, 0, 255]);
    assert!(h.errors().is_empty(), "{:?}", h.errors());
}

/// WebGL：清屏 + 一个三角形（着色器编译链接、uniform 位置走程序信息缓存）、rAF 每帧上屏
#[test]
fn plus_webgl_三角形与程序信息() {
    let _g = serial();
    let js = r#"
var T = require('_lib/t.js');
var c = wx.createCanvas(), gl = c.getContext('webgl');
function sh(t, s) { var x = gl.createShader(t); gl.shaderSource(x, s); gl.compileShader(x); return x; }
var p = gl.createProgram();
gl.attachShader(p, sh(gl.VERTEX_SHADER, 'attribute vec2 pos; void main(){ gl_Position = vec4(pos, 0.0, 1.0); }'));
gl.attachShader(p, sh(gl.FRAGMENT_SHADER, 'precision mediump float; uniform vec4 col; uniform float arr[3]; void main(){ gl_FragColor = col + vec4(arr[2] * 0.0); }'));
gl.linkProgram(p);
var linked = gl.getProgramParameter(p, gl.LINK_STATUS), nu = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS);
var loc = gl.getUniformLocation(p, 'col'), la = gl.getUniformLocation(p, 'arr[2]'), apos = gl.getAttribLocation(p, 'pos');
var b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b);
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1]), gl.STATIC_DRAW);
var frames = 0;
function f() {
  frames++;
  gl.viewport(0, 0, c.width, c.height);
  gl.clearColor(0, 0, 1, 1); gl.clear(gl.COLOR_BUFFER_BIT);
  gl.useProgram(p); gl.uniform4f(loc, 0, 1, 0, 1);
  gl.enableVertexAttribArray(apos); gl.vertexAttribPointer(apos, 2, gl.FLOAT, false, 0, 0);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
  if (frames === 10) T.ev('gl', { linked: linked, nu: nu, loc: !!loc, la: !!la, apos: apos, err: gl.getError(), max: gl.getParameter(gl.MAX_TEXTURE_SIZE) > 0,
    bound: gl.getParameter(gl.ARRAY_BUFFER_BINDING) === b, ext: Array.isArray(gl.getSupportedExtensions()) });
  requestAnimationFrame(f);
}
requestAnimationFrame(f);
"#;
    let mut h = open!(WebHarness::open_inline("plusgl", "{}", js, &[], plus()));
    h.until("T:EV gl", 15.0).sleep(300);
    let e = &h.events("gl")[0];
    assert_eq!(e, &serde_json::json!({ "linked": true, "nu": 2, "loc": true, "la": true, "apos": 0, "err": 0, "max": true, "bound": true, "ext": true }));
    assert_eq!(h.native_pixel(40, 600), [0, 255, 0], "左下角的三角形");
    assert_eq!(h.native_pixel(360, 20), [0, 0, 255], "右上角是清屏色");
    assert!(h.errors().is_empty(), "{:?}", h.errors());
}

/// 图片：原生解码、onload 经渲染端回到页面、drawImage 与 WebGL 纹理都用得上
#[test]
fn plus_图片_解码在渲染端() {
    let _g = serial();
    let mut h = open!(WebHarness::open("g-image", WebOpts { plus: true, query: "http=1", ..Default::default() }));
    h.until("T:EV drawn", 15.0).sleep(300);
    let loads = h.events("load");
    for k in ["png", "jpg", "webp", "gif", "data", "wxfile"] {
        assert!(loads.iter().any(|e| e["k"] == k), "{k} 没加载：{loads:?}");
    }
    assert!(h.events("error").iter().any(|e| e["k"] == "missing"));
    assert!(h.native_colors() > 3, "drawImage 画到了渲染端的主画布上");
}

/// 第三方引擎：PixiJS（WebGL）与 Phaser（WebGL / Canvas）经适配层跑，像素在渲染端、点击照常
#[test]
fn plus_引擎_pixi_phaser_three() {
    let _g = serial();
    let have = |f: &str, l: &str| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sample-game").join(f).join("libs").join(l).exists();
    if have("e-pixi", "pixi.js") {
        let mut h = open!(WebHarness::open("e-pixi", WebOpts { plus: true, query: "dpr=1", ..Default::default() }));
        h.until("T:EV frames", 30.0).sleep(300);
        assert!(h.errors().is_empty(), "{:?}", h.errors());
        assert_eq!(h.native_pixel(100, 520), [0, 255, 0]);
        h.tap(100.0, 520.0);
        h.until("T:EV tap", 5.0).sleep(300);
        assert_eq!(h.native_pixel(100, 520), [255, 0, 0], "点了之后变红");
        assert_eq!(h.native_pixel(140, 180), [240, 160, 32]);
    }
    if have("e-phaser", "phaser.js") {
        for q in ["type=webgl", "type=canvas"] {
            let mut h = open!(WebHarness::open("e-phaser", WebOpts { plus: true, query: q, ..Default::default() }));
            h.until("T:EV frames", 30.0).sleep(300);
            assert!(h.errors().is_empty(), "{q}：{:?}", h.errors());
            assert_eq!(h.native_pixel(140, 180), [240, 160, 32], "{q}");
        }
    }
    if have("e-three", "three.js") {
        let mut h = open!(WebHarness::open("e-three", WebOpts { plus: true, query: "frames=30", ..Default::default() }));
        h.until("T:EV done", 30.0).sleep(300);
        assert!(h.errors().is_empty(), "{:?}", h.errors());
        assert!(h.native_colors() > 200, "光照渐变：{}", h.native_colors());
    }
}

/// 示例游戏在高性能+ 下冷启动：5 秒内没有报错、渲染端画出了东西
#[test]
fn plus_示例游戏都起得来() {
    let _g = serial();
    for g in ["play-2048", "play-flappy", "play-snake", "play-breakout", "play-jump", "play-fighter", "play-3d-stack", "play-3d-runner", "play-3d-bowling", "play-3d-island"] {
        if !std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sample-game").join(g).is_dir() { continue; }
        let mut h = open!(WebHarness::open(g, WebOpts { plus: true, query: "auto=1", ..Default::default() }));
        h.sleep(6000);
        let errs = h.errors();
        assert!(errs.is_empty(), "{g} 报错：{errs:?}\n{:?}", h.log.lock().unwrap().iter().rev().take(20).collect::<Vec<_>>());
        let n = h.native_colors();
        assert!(n > 20, "{g} 渲染端颜色数 {n}（黑屏？）\n{:?}", h.log.lock().unwrap().iter().rev().take(20).collect::<Vec<_>>());
        eprintln!("✓ {g}（高性能+）：{n} 种颜色");
    }
}
