//! GT-WG（G8 WebGL）：无头走 ANGLE + SwiftShader（确定性），判像素与 WebGL 语义（错误码、对象归属、丢失恢复、读回）。
//! 要 `third_party/angle/<平台>/`（tools/fetch-angle.sh）；没有就整组失败并说明怎么拿。
use super::*;

fn px(h: &mut GameHarness, x: u32, y: u32) -> [u8; 3] {
    let (w, _, p) = h.engine.frame_rgba();
    let i = ((y * w + x) * 4) as usize;
    [p[i], p[i + 1], p[i + 2]]
}

/// 每个用例共用的着色器小工具（JS）
const LIB: &str = r#"
var T = require('_lib/t.js');
function prog(gl, vs, fs) {
    function sh(t, s) { var x = gl.createShader(t); gl.shaderSource(x, s); gl.compileShader(x); if (!gl.getShaderParameter(x, gl.COMPILE_STATUS)) T.ev('shaderErr', { log: gl.getShaderInfoLog(x) }); return x; }
    var p = gl.createProgram(); gl.attachShader(p, sh(gl.VERTEX_SHADER, vs)); gl.attachShader(p, sh(gl.FRAGMENT_SHADER, fs));
    gl.bindAttribLocation(p, 0, 'p'); gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) T.ev('linkErr', { log: gl.getProgramInfoLog(p) });
    return p;
}
var VS = 'attribute vec2 p; varying vec2 uv; void main(){ uv = p * 0.5 + 0.5; gl_Position = vec4(p, 0.0, 1.0); }';
function quad(gl, x0, y0, x1, y1) {
    var b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([x0, y0, x1, y0, x0, y1, x1, y1]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    return b;
}
"#;

fn open(name: &str, body: &str) -> GameHarness {
    let js = format!("{LIB}\n{body}");
    let mut h = GameHarness::open_inline(name, &js, Opts::default());
    h.vsyncs(3);
    assert!(!h.has("返回 null"), "WebGL 建不起来（先跑 tools/fetch-angle.sh）：{:?}", h.log.iter().filter(|l| l.contains("WebGL")).collect::<Vec<_>>());
    h
}

/// WG1：建上下文、清屏、画一个三角形；属性、版本串、对象类、后端
#[test]
fn wg1_清屏与三角形() {
    let _g = serial();
    let mut h = open("wg1", r#"
var c = wx.createCanvas(), gl = c.getContext('webgl');
T.ev('ctx', { ok: !!gl, same: c.getContext('experimental-webgl') === gl, twoD: c.getContext('2d'), isGL: gl instanceof WebGLRenderingContext,
    attrs: gl.getContextAttributes(), ver: gl.getParameter(gl.VERSION), maxTex: gl.getParameter(gl.MAX_TEXTURE_SIZE),
    vp: Array.prototype.slice.call(gl.getParameter(gl.VIEWPORT)), dbw: gl.drawingBufferWidth, err: gl.getError(), lost: gl.isContextLost(),
    buf: gl.createBuffer() instanceof WebGLBuffer, exts: gl.getSupportedExtensions().length });
var p = prog(gl, 'attribute vec2 p; void main(){ gl_Position = vec4(p, 0.0, 1.0); }', 'precision mediump float; uniform vec4 col; void main(){ gl_FragColor = col; }');
var b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b);
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, 0, 1]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
function frame() {
    gl.clearColor(1, 0, 0, 1); gl.clear(gl.COLOR_BUFFER_BIT);
    gl.useProgram(p); gl.uniform4f(gl.getUniformLocation(p, 'col'), 0, 1, 0, 1);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    var px = new Uint8Array(4); gl.readPixels(c.width >> 1, c.height >> 2, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
    T.ev('read', { px: Array.prototype.slice.call(px), err: gl.getError() });
}
requestAnimationFrame(frame);
T.ready('wg1');
"#);
    h.assert_no_fail();
    let c = &h.events("ctx")[0];
    assert_eq!((c["ok"].as_bool(), c["same"].as_bool(), c["twoD"].is_null(), c["isGL"].as_bool()), (Some(true), Some(true), true, Some(true)), "{c}");
    assert_eq!(c["ver"], "WebGL 1.0 (OpenGL ES 2.0 Chromium)");
    assert_eq!(c["attrs"]["antialias"], true);
    assert_eq!(c["attrs"]["preserveDrawingBuffer"], false);
    assert!(c["attrs"].get("__backend").is_none(), "内部字段不外露：{c}");
    assert!(c["maxTex"].as_u64().unwrap() >= 4096);
    assert_eq!(c["vp"], serde_json::json!([0, 0, 375, 667]));
    assert_eq!((c["err"].as_u64(), c["lost"].as_bool(), c["buf"].as_bool()), (Some(0), Some(false), Some(true)));
    assert!(c["exts"].as_u64().unwrap() > 5, "{c}");
    assert_eq!(h.engine.webgl_backend(), Some("swiftshader"));
    let r = &h.events("read")[0];
    assert_eq!(r["px"], serde_json::json!([0, 255, 0, 255]), "{r}");
    assert_eq!(r["err"], 0);
    // 上屏：中间是三角形的绿，左上角是清屏的红（GL 的 y 向上，读回时翻过来了）
    assert_eq!(px(&mut h, 187, 400), [0, 255, 0]);
    assert_eq!(px(&mut h, 5, 5), [255, 0, 0]);
    assert!(h.errors.is_empty(), "{:?}", h.errors);
}

/// WG2：纹理来源 —— Uint8Array（FLIP_Y）、2D 画布、ImageData；采样到屏上
#[test]
fn wg2_纹理来源() {
    let _g = serial();
    let mut h = open("wg2", r#"
var c = wx.createCanvas(), gl = c.getContext('webgl');
var p = prog(gl, VS, 'precision mediump float; varying vec2 uv; uniform sampler2D t; void main(){ gl_FragColor = texture2D(t, uv); }');
quad(gl, -1, -1, 1, 1);
function tex() { var t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE); return t; }
// 1×2：上红下蓝（数据第 0 行 = 红）。FLIP_Y 之后第 0 行（纹理底部，uv.y=0）变成蓝
var t1 = tex();
gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 2, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([255, 0, 0, 255, 0, 0, 255, 255]));
gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
// 2D 画布：左黄右青
var c2 = wx.createCanvas(); c2.width = 2; c2.height = 1; var g2 = c2.getContext('2d');
g2.fillStyle = '#ffff00'; g2.fillRect(0, 0, 1, 1); g2.fillStyle = '#00ffff'; g2.fillRect(1, 0, 1, 1);
var t2 = tex(); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, c2);
// ImageData：一个品红像素
var t3 = tex(); gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, { width: 1, height: 1, data: new Uint8ClampedArray([255, 0, 255, 255]) });
var which = 0;
function frame() {
    gl.useProgram(p);
    gl.bindTexture(gl.TEXTURE_2D, [t1, t2, t3][which]);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    T.ev('drawn', { which: which, err: gl.getError() });
}
globalThis.show = function(i) { which = i; requestAnimationFrame(frame); };
show(0);
T.ready('wg2');
"#);
    h.assert_no_fail();
    // GL 的 uv.y = 0 在屏幕底部
    assert_eq!(px(&mut h, 187, 600), [0, 0, 255], "FLIP_Y 后底部是数据的第 1 行（蓝）");
    assert_eq!(px(&mut h, 187, 60), [255, 0, 0]);
    h.engine.eval("show(1)").unwrap();
    h.vsyncs(2);
    // 画布来源：第 0 行在顶上（DOM 来源不翻转时，画布顶 = 纹理第 0 行 = uv.y 0 = 屏幕底）—— 只有一行，左右分色
    assert_eq!(px(&mut h, 40, 300), [255, 255, 0]);
    assert_eq!(px(&mut h, 330, 300), [0, 255, 255]);
    h.engine.eval("show(2)").unwrap();
    h.vsyncs(2);
    assert_eq!(px(&mut h, 187, 300), [255, 0, 255]);
    assert!(h.events("drawn").iter().all(|e| e["err"] == 0), "{:?}", h.events("drawn"));
}

/// WG3：WebGL 的错误语义 —— 越界绘制、别的上下文的对象、已删对象、readPixels 目标太短、类型不匹配、uniform 位置不属于当前程序
#[test]
fn wg3_错误语义() {
    let _g = serial();
    let h = open("wg3", r#"
var gl = wx.createCanvas().getContext('webgl'), gl2 = wx.createCanvas().getContext('webgl');
var p = prog(gl, 'attribute vec2 p; void main(){ gl_Position = vec4(p, 0.0, 1.0); }', 'precision mediump float; uniform float k; void main(){ gl_FragColor = vec4(k); }');
var q = prog(gl, 'attribute vec2 p; void main(){ gl_Position = vec4(p, 0.0, 1.0); }', 'precision mediump float; uniform float k; void main(){ gl_FragColor = vec4(k); }');
var r = {};
var b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(6), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0); gl.useProgram(p);
gl.drawArrays(gl.TRIANGLES, 0, 3); r.ok = gl.getError();
gl.drawArrays(gl.TRIANGLES, 0, 4); r.oob = gl.getError();
gl.bindBuffer(gl.ARRAY_BUFFER, gl2.createBuffer()); r.foreign = gl.getError();
var d = gl.createTexture(); gl.deleteTexture(d); gl.bindTexture(gl.TEXTURE_2D, d); r.deleted = gl.getError();
r.isDeleted = gl.isTexture(d);
gl.readPixels(0, 0, 2, 2, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(15)); r.short = gl.getError();
gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, new Float32Array(4)); r.mismatch = gl.getError();
var lq = gl.getUniformLocation(q, 'k'); gl.uniform1f(lq, 1); r.otherProg = gl.getError();
gl.uniform1f(null, 1); r.nullLoc = gl.getError();
gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 2, 2, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(8)); r.texShort = gl.getError();
gl.getParameter(0x1234); r.badEnum = gl.getError();
r.twice = [gl.getError(), gl.getError()];
try { gl.bindBuffer(gl.ARRAY_BUFFER, {}); r.typeErr = false; } catch (e) { r.typeErr = e instanceof TypeError; }
gl.lineWidth(0); r.line0 = gl.getError();
T.ev('r', r);
T.ready('wg3');
"#);
    h.assert_no_fail();
    let r = &h.events("r")[0];
    let want = serde_json::json!({ "ok": 0, "oob": 0x502, "foreign": 0x502, "deleted": 0x502, "isDeleted": false, "short": 0x502,
        "mismatch": 0x502, "otherProg": 0x502, "nullLoc": 0, "texShort": 0x502, "badEnum": 0x500, "twice": [0, 0], "typeErr": true, "line0": 0x501 });
    assert_eq!(r, &want);
}

/// WG4：WEBGL_lose_context —— 丢失事件、getError 报一次 CONTEXT_LOST_WEBGL、丢失期间 create* 给无效对象（不是 null，新版规范）、preventDefault 后 restoreContext、旧对象作废
#[test]
fn wg4_丢失与恢复() {
    let _g = serial();
    let mut h = open("wg4", r#"
var c = wx.createCanvas(), gl = c.getContext('webgl'), ext = gl.getExtension('WEBGL_lose_context');
var old = gl.createBuffer();
c.addEventListener('webglcontextlost', function (e) { e.preventDefault(); T.ev('lost', { isLost: gl.isContextLost(), err1: gl.getError(), err2: gl.getError(), create: (function () { var b = gl.createBuffer(); return b instanceof WebGLBuffer && !gl.isBuffer(b); })(), param: gl.getParameter(gl.MAX_TEXTURE_SIZE) }); ext.restoreContext(); });
c.addEventListener('webglcontextrestored', function () {
    gl.bindBuffer(gl.ARRAY_BUFFER, old);
    T.ev('restored', { isLost: gl.isContextLost(), oldErr: gl.getError(), fresh: gl.createBuffer() instanceof WebGLBuffer });
    gl.clearColor(0, 0, 1, 1); gl.clear(gl.COLOR_BUFFER_BIT);
});
T.ev('has', { ext: !!ext, again: gl.getExtension('webgl_lose_context') === ext });
ext.loseContext();
T.ready('wg4');
"#);
    h.vsyncs(4);
    h.assert_no_fail();
    assert_eq!(h.events("has")[0], serde_json::json!({ "ext": true, "again": true }));
    let l = &h.events("lost")[0];
    assert_eq!(l, &serde_json::json!({ "isLost": true, "err1": 0x9242, "err2": 0, "create": true, "param": null }));
    let r = &h.events("restored")[0];
    assert_eq!(r, &serde_json::json!({ "isLost": false, "oldErr": 0x502, "fresh": true }));
    assert_eq!(px(&mut h, 10, 10), [0, 0, 255], "恢复后画的蓝色上屏");
}

/// WG5：preserveDrawingBuffer=false —— 上屏之后不画的那一帧屏上仍是上一帧，readPixels 读到的是清过的 0；true 时保留
#[test]
fn wg5_绘图缓冲保留() {
    let _g = serial();
    let mut h = open("wg5", r#"
var c = wx.createCanvas(), gl = c.getContext('webgl', { preserveDrawingBuffer: !!T.q.keep });
gl.clearColor(0, 1, 0, 1); gl.clear(gl.COLOR_BUFFER_BIT);
var n = 0;
function frame() {
    if (++n === 3) { var px = new Uint8Array(4); gl.readPixels(1, 1, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px); T.ev('later', { px: Array.prototype.slice.call(px) }); return; }
    requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
T.ready('wg5');
"#);
    h.vsyncs(4);
    assert_eq!(h.events("later")[0]["px"], serde_json::json!([0, 0, 0, 0]));
    assert_eq!(px(&mut h, 10, 10), [0, 255, 0], "屏上保持上屏的那一帧");
    let js = format!("{LIB}\n{}", r#"
var c = wx.createCanvas(), gl = c.getContext('webgl', { preserveDrawingBuffer: true });
gl.clearColor(0, 1, 0, 1); gl.clear(gl.COLOR_BUFFER_BIT);
var n = 0;
function frame() { if (++n === 3) { var px = new Uint8Array(4); gl.readPixels(1, 1, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px); T.ev('later', { px: Array.prototype.slice.call(px) }); return; } requestAnimationFrame(frame); }
requestAnimationFrame(frame);
T.ready('wg5b');
"#);
    let mut k = GameHarness::open_inline("wg5b", &js, Opts::default());
    k.vsyncs(6);
    assert_eq!(k.events("later")[0]["px"], serde_json::json!([0, 255, 0, 255]));
}

/// WG6：WebGL2 —— VAO、实例化、uniform 块、3D 纹理、整数 uniform、getBufferSubData
#[test]
fn wg6_webgl2() {
    let _g = serial();
    let mut h = open("wg6", r#"
var c = wx.createCanvas(), gl = c.getContext('webgl2');
var r = { is2: gl instanceof WebGL2RenderingContext, ver: gl.getParameter(gl.VERSION), noGL1: c.getContext('webgl') };
function sh(t, s) { var x = gl.createShader(t); gl.shaderSource(x, s); gl.compileShader(x); if (!gl.getShaderParameter(x, gl.COMPILE_STATUS)) r.log = gl.getShaderInfoLog(x); return x; }
var p = gl.createProgram();
gl.attachShader(p, sh(gl.VERTEX_SHADER, '#version 300 es\nlayout(location=0) in vec2 p; layout(location=1) in float off; void main(){ gl_Position = vec4(p.x * 0.2 + off, p.y * 0.2, 0.0, 1.0); }'));
gl.attachShader(p, sh(gl.FRAGMENT_SHADER, '#version 300 es\nprecision mediump float; layout(std140) uniform U { vec4 col; }; out vec4 o; void main(){ o = col; }'));
gl.linkProgram(p); r.link = gl.getProgramParameter(p, gl.LINK_STATUS);
var vao = gl.createVertexArray(); gl.bindVertexArray(vao);
var b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
var ib = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, ib); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-0.5, 0.5]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(1); gl.vertexAttribPointer(1, 1, gl.FLOAT, false, 0, 0); gl.vertexAttribDivisor(1, 1);
gl.bindVertexArray(null);
var ub = gl.createBuffer(); gl.bindBuffer(gl.UNIFORM_BUFFER, ub); gl.bufferData(gl.UNIFORM_BUFFER, new Float32Array([1, 0.5, 0, 1]), gl.DYNAMIC_DRAW);
var bi = gl.getUniformBlockIndex(p, 'U'); gl.uniformBlockBinding(p, bi, 0); gl.bindBufferBase(gl.UNIFORM_BUFFER, 0, ub);
r.blockSize = gl.getActiveUniformBlockParameter(p, bi, gl.UNIFORM_BLOCK_DATA_SIZE);
var back = new Float32Array(4); gl.getBufferSubData(gl.UNIFORM_BUFFER, 0, back); r.back = Array.prototype.slice.call(back);
var t3 = gl.createTexture(); gl.bindTexture(gl.TEXTURE_3D, t3);
gl.texImage3D(gl.TEXTURE_3D, 0, gl.RGBA8, 2, 2, 2, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array(32)); r.tex3d = gl.getError();
r.vaoBound = gl.getParameter(gl.VERTEX_ARRAY_BINDING) === null;
gl.bindVertexArray(vao); r.vaoGet = gl.getParameter(gl.VERTEX_ARRAY_BINDING) === vao;
function frame() {
    gl.clearColor(0, 0, 0, 1); gl.clear(gl.COLOR_BUFFER_BIT);
    gl.useProgram(p); gl.bindVertexArray(vao); gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, 2);
    r.err = gl.getError();
    T.ev('r', r);
}
requestAnimationFrame(frame);
T.ready('wg6');
"#);
    h.assert_no_fail();
    let r = &h.events("r")[0];
    assert_eq!((r["is2"].as_bool(), r["link"].as_bool(), r["noGL1"].is_null(), r["err"].as_u64(), r["tex3d"].as_u64()), (Some(true), Some(true), true, Some(0), Some(0)), "{r}");
    assert_eq!(r["ver"], "WebGL 2.0 (OpenGL ES 3.0 Chromium)");
    assert_eq!(r["blockSize"], 16);
    assert_eq!(r["back"], serde_json::json!([1, 0.5, 0, 1]));
    assert_eq!((r["vaoBound"].as_bool(), r["vaoGet"].as_bool()), (Some(true), Some(true)));
    // 两个实例：x 偏 -0.5 与 +0.5（屏宽 375：中心 94 与 281），中间是黑
    let orange = [255, 128, 0];
    assert_eq!(px(&mut h, 94, 333), orange);
    assert_eq!(px(&mut h, 281, 333), orange);
    assert_eq!(px(&mut h, 187, 333), [0, 0, 0]);
}

/// WG7：webgl:"off"（等同 MINI_WEBGL=off）→ getContext 返回 null、日志写明原因；2D 照常
#[test]
fn wg7_关掉webgl() {
    let _g = serial();
    let mut h = GameHarness::open_inline("wg7", r#"
var T = require('_lib/t.js');
var c = wx.createCanvas();
T.ev('r', { gl: c.getContext('webgl'), gl2: wx.createCanvas().getContext('webgl2'), twoD: !!wx.createCanvas().getContext('2d') });
T.ready('wg7');
"#, Opts { webgl: Some("off"), ..Default::default() });
    h.vsyncs(1);
    assert_eq!(h.events("r")[0], serde_json::json!({ "gl": null, "gl2": null, "twoD": true }));
    assert!(h.has("返回 null"), "{:?}", h.log);
}

/// WG8：WebGL 画布当图片源 —— drawImage 到 2D 画布、toDataURL、wxBindCanvasTexture（2D 画布 → 纹理）
#[test]
fn wg8_画布互通() {
    let _g = serial();
    let mut h = open("wg8", r#"
var main = wx.createCanvas(), g = main.getContext('2d');
var off = wx.createCanvas(); off.width = 4; off.height = 4;
var gl = off.getContext('webgl');
gl.clearColor(0, 0, 1, 1); gl.clear(gl.COLOR_BUFFER_BIT);
g.fillStyle = '#ff0000'; g.fillRect(0, 0, main.width, main.height);
g.drawImage(off, 0, 0, 100, 100);
var url = off.toDataURL();
// wxBindCanvasTexture：2D 画布（2×2 绿）→ 纹理，画回第二块 WebGL 画布再读
var src = wx.createCanvas(); src.width = 2; src.height = 2; var sg = src.getContext('2d'); sg.fillStyle = '#00ff00'; sg.fillRect(0, 0, 2, 2);
var o2 = wx.createCanvas(); o2.width = 2; o2.height = 2; var g2 = o2.getContext('webgl');
var p = prog(g2, VS, 'precision mediump float; varying vec2 uv; uniform sampler2D t; void main(){ gl_FragColor = texture2D(t, uv); }');
quad(g2, -1, -1, 1, 1);
var t = g2.createTexture(); g2.bindTexture(g2.TEXTURE_2D, t); g2.texParameteri(g2.TEXTURE_2D, g2.TEXTURE_MIN_FILTER, g2.NEAREST);
g2.wxBindCanvasTexture(g2.TEXTURE_2D, src);
g2.useProgram(p); g2.drawArrays(g2.TRIANGLE_STRIP, 0, 4);
var px = new Uint8Array(4); g2.readPixels(0, 0, 1, 1, g2.RGBA, g2.UNSIGNED_BYTE, px);
T.ev('r', { png: url.indexOf('data:image/png;base64,') === 0 && url.length > 60, bound: Array.prototype.slice.call(px), err: g2.getError() });
T.ready('wg8');
"#);
    h.assert_no_fail();
    let r = &h.events("r")[0];
    assert_eq!(r, &serde_json::json!({ "png": true, "bound": [0, 255, 0, 255], "err": 0 }));
    assert_eq!(px(&mut h, 50, 50), [0, 0, 255], "WebGL 画布 drawImage 到 2D 主画布");
    assert_eq!(px(&mut h, 200, 200), [255, 0, 0]);
}
