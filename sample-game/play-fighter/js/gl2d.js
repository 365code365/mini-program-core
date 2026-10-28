// 用 WebGL 画 2D：矩形、圆角矩形、圆 / 椭圆、凸多边形、粗线、描边、渐变都拆成三角形，攒成一批一次画；
// 文字按**单个字符**写在离屏 2D 画布上、拷进同一张图集纹理，一段字 = 一串字符贴图（纯色形状采样图集里的一块白，不打断批次）。
// 四个 play-* 2D 游戏共用（各拷一份，包自包含）。play-fighter 这份另加了贴图：image()（图集里的一块，可翻转 / 染色）、
// 叠加混合（setBlend('add')：顶点 alpha 记 0，预乘混合 ONE, ONE_MINUS_SRC_ALPHA 就成了相加，不用切 blendFunc、不打断批次）
// 与剪影（tint 给 [0,0,0,a]：贴图 rgb 乘 0、只留覆盖率，画影子 / 倒影用）。换贴图才 flush。
//
// 坐标是逻辑像素（windowWidth × windowHeight），左上角原点，与 Canvas 2D 一致；save / restore / translate / rotate / scale / alpha 也照它。
// 颜色收 '#rgb' '#rrggbb' 'rgb()' 'rgba()' 或 [r, g, b, a]（0~1）。着色器输出预乘 alpha，混合用 ONE, ONE_MINUS_SRC_ALPHA。
//
// 性能取舍（一加 PLY110 真机，每帧都记的 [GAME] 日志）：
// - 每帧的时间几乎全在 JS 生成顶点上（约 2.5ms / 千顶点），上屏是 0。所以：顶点函数不分配、不查表；颜色每个形状设一次；
//   圆用预算好的 cos/sin 表；**不变的东西录成图层 / 精灵**（layer / sprite：录一次进自己的顶点缓冲，之后每帧一次 drawArrays，
//   位置、缩放、透明度走 uniform —— 水管、云、2048 的方块都这样画，每帧不再在 JS 里拼它们的顶点）。
// - 文字按字符缓存：分数从 9 变 10 只是多贴一个字符，不重新光栅化整段（整段重画在真机上一次 20~30ms，就是那一下顿）。
//   描边字用 16 个方向偏移的 fillText（本引擎的 strokeText 一次几百毫秒）。
// - 一段字一张纹理时每段字前后都要换纹理 flush（2048 一帧 ~50 次 draw，无头每帧 23ms）；图集之后一帧通常 1 次 draw + 图层数。
var VS = [
  'attribute vec2 a_p; attribute vec4 a_c; attribute vec2 a_uv;',
  'uniform vec2 u_size; uniform vec2 u_off; uniform float u_scale; uniform float u_alpha; varying vec4 v_c; varying vec2 v_uv;',
  'void main() { vec2 p = a_p * u_scale + u_off; v_c = a_c * u_alpha; v_uv = a_uv; gl_Position = vec4(p.x / u_size.x * 2.0 - 1.0, 1.0 - p.y / u_size.y * 2.0, 0.0, 1.0); }'
].join('\n');
var FS = 'precision mediump float; varying vec4 v_c; varying vec2 v_uv; uniform sampler2D u_t; void main() { gl_FragColor = texture2D(u_t, v_uv) * v_c; }';
var FLOATS = 8, MAX_VERTS = 30000;               // 每顶点 x y r g b a u v
var TAU = Math.PI * 2;

// 单位圆的 cos / sin 表：按段数缓存
var circleTables = {};
function unitCircle(k) {
  var t = circleTables[k];
  if (t) return t;
  t = new Float32Array((k + 1) * 2);
  for (var i = 0; i <= k; i++) { t[i * 2] = Math.cos(TAU * i / k); t[i * 2 + 1] = Math.sin(TAU * i / k); }
  return (circleTables[k] = t);
}

function make(canvas, W, H, dpr) {
  canvas.width = Math.round(W * dpr); canvas.height = Math.round(H * dpr);
  var gl = canvas.getContext('webgl', { antialias: true, alpha: false, depth: false, stencil: false, preserveDrawingBuffer: false });
  if (!gl) throw new Error('这台设备拿不到 WebGL 上下文');
  function sh(t, s) { var x = gl.createShader(t); gl.shaderSource(x, s); gl.compileShader(x); if (!gl.getShaderParameter(x, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(x)); return x; }
  var prog = gl.createProgram();
  gl.attachShader(prog, sh(gl.VERTEX_SHADER, VS)); gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FS)); gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
  gl.useProgram(prog);
  var aP = gl.getAttribLocation(prog, 'a_p'), aC = gl.getAttribLocation(prog, 'a_c'), aUV = gl.getAttribLocation(prog, 'a_uv');
  var uOff = gl.getUniformLocation(prog, 'u_off'), uScale = gl.getUniformLocation(prog, 'u_scale'), uAlpha = gl.getUniformLocation(prog, 'u_alpha');
  gl.uniform2f(gl.getUniformLocation(prog, 'u_size'), W, H);
  gl.uniform2f(uOff, 0, 0); gl.uniform1f(uScale, 1); gl.uniform1f(uAlpha, 1);
  var vbo = gl.createBuffer();
  function pointers() {
    gl.vertexAttribPointer(aP, 2, gl.FLOAT, false, FLOATS * 4, 0);
    gl.vertexAttribPointer(aC, 4, gl.FLOAT, false, FLOATS * 4, 8);
    gl.vertexAttribPointer(aUV, 2, gl.FLOAT, false, FLOATS * 4, 24);
  }
  gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
  gl.bufferData(gl.ARRAY_BUFFER, MAX_VERTS * FLOATS * 4, gl.DYNAMIC_DRAW);
  gl.enableVertexAttribArray(aP); gl.enableVertexAttribArray(aC); gl.enableVertexAttribArray(aUV);
  pointers();
  gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);

  // 图集：左上角 4×4 是白色（纯色形状采样它的中心），其余按行装字符。装满就整张作废重装（字符下一帧按需重画）
  var ATLAS = Math.min(2048, gl.getParameter(gl.MAX_TEXTURE_SIZE) || 2048);
  var atlas = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, atlas);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, ATLAS, ATLAS, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
  var WHITE_PX = new Uint8Array(64); for (var wi = 0; wi < 64; wi++) WHITE_PX[wi] = 255;
  var WU = 2 / ATLAS;                                // 白块中心的纹理坐标
  var shelfX, shelfY, shelfH, layers = {};
  function resetAtlas() {
    gl.bindTexture(gl.TEXTURE_2D, atlas);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, 4, 4, gl.RGBA, gl.UNSIGNED_BYTE, WHITE_PX);
    shelfX = 8; shelfY = 0; shelfH = 8;
    for (var k in layers) layers[k].sig = null;     // 图层里可能录了字：纹理坐标作废，下次重录
  }
  resetAtlas();
  var scratch = wx.createCanvas();                   // 所有字符共用一块离屏画布：写好、拷进图集

  // ───────── 顶点批 ─────────
  var buf = new Float32Array(MAX_VERTS * FLOATS), n = 0, draws = 0, verts = 0, recording = false;
  var m0 = 1, m1 = 0, m2 = 0, m3 = 1, m4 = 0, m5 = 0, alpha = 1, stack = [];
  var cr = 0, cg = 0, cb = 0, ca = 1;                 // 当前顶点色（已乘 alpha、已预乘）
  var additive = false, curTex = atlas;
  // 纯色形状与文字采样图集：当前绑的是别的贴图就先把攒着的画掉再换回来
  function useTex(t) { if (curTex !== t) { flush(); gl.bindTexture(gl.TEXTURE_2D, t); curTex = t; } }
  // GL 状态按需改：每次调用都要过一次 JS↔原生，连着画的同种精灵不重复绑缓冲 / 设 uniform
  var bound = vbo, uo0 = 0, uo1 = 0, us = 1, ua = 1;
  function bindBuf(b) { if (bound !== b) { gl.bindBuffer(gl.ARRAY_BUFFER, b); pointers(); bound = b; } }
  function uniforms(ox, oy, sc, a) {
    if (ox !== uo0 || oy !== uo1) { gl.uniform2f(uOff, ox, oy); uo0 = ox; uo1 = oy; }
    if (sc !== us) { gl.uniform1f(uScale, sc); us = sc; }
    if (a !== ua) { gl.uniform1f(uAlpha, a); ua = a; }
  }
  function flush() {
    if (!n) return;
    if (recording) throw new Error('gl2d：图层太大（超过 ' + MAX_VERTS + ' 个顶点）');
    bindBuf(vbo); uniforms(0, 0, 1, 1);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, buf.subarray(0, n * FLOATS));
    gl.drawArrays(gl.TRIANGLES, 0, n);
    draws++; verts += n;
    n = 0;
  }
  function room(k) { if (n + k > MAX_VERTS) flush(); }
  function setC(c) { if (curTex !== atlas) useTex(atlas); var a = c[3] * alpha; cr = c[0] * a; cg = c[1] * a; cb = c[2] * a; ca = additive ? 0 : a; }
  function v(x, y) {
    var o = n * FLOATS;
    buf[o] = m0 * x + m2 * y + m4; buf[o + 1] = m1 * x + m3 * y + m5;
    buf[o + 2] = cr; buf[o + 3] = cg; buf[o + 4] = cb; buf[o + 5] = ca; buf[o + 6] = WU; buf[o + 7] = WU;
    n++;
  }
  function vt(x, y, u, w) { v(x, y); buf[(n - 1) * FLOATS + 6] = u; buf[(n - 1) * FLOATS + 7] = w; }
  function quad(x0, y0, x1, y1, x2, y2, x3, y3) { room(6); v(x0, y0); v(x1, y1); v(x2, y2); v(x0, y0); v(x2, y2); v(x3, y3); }

  // ───────── 颜色 ─────────
  var colorCache = {};
  function col(c) {
    if (typeof c !== 'string') return c.length === 3 ? [c[0], c[1], c[2], 1] : c;
    var hit = colorCache[c]; if (hit) return hit;
    var out = [0, 0, 0, 1], s = c.trim(), mm;
    if (s[0] === '#') {
      var h = s.slice(1);
      if (h.length === 3) h = h[0] + h[0] + h[1] + h[1] + h[2] + h[2];
      var num = parseInt(h.slice(0, 6), 16);
      out = [(num >> 16 & 255) / 255, (num >> 8 & 255) / 255, (num & 255) / 255, h.length === 8 ? parseInt(h.slice(6), 16) / 255 : 1];
    } else if ((mm = s.match(/^rgba?\(([^)]*)\)$/))) {
      var p = mm[1].split(',').map(function (x) { return parseFloat(x); });
      out = [p[0] / 255, p[1] / 255, p[2] / 255, p.length > 3 ? p[3] : 1];
    }
    colorCache[c] = out;
    return out;
  }

  // ───────── 形状 ─────────
  function rect(x, y, w, h, c) { setC(col(c)); quad(x, y, x + w, y, x + w, y + h, x, y + h); }
  // 渐变矩形：vertical=true 上→下，否则左→右；stops = [[t, 颜色], …]
  function gradRect(x, y, w, h, stops, vertical) {
    for (var i = 0; i + 1 < stops.length; i++) {
      var a = col(stops[i][1]), b = col(stops[i + 1][1]), t0 = stops[i][0], t1 = stops[i + 1][0];
      room(6);
      if (vertical) {
        var y0 = y + h * t0, y1 = y + h * t1;
        setC(a); v(x, y0); v(x + w, y0); setC(b); v(x + w, y1); setC(a); v(x, y0); setC(b); v(x + w, y1); v(x, y1);
      } else {
        var x0 = x + w * t0, x1 = x + w * t1;
        setC(a); v(x0, y); setC(b); v(x1, y); v(x1, y + h); setC(a); v(x0, y); setC(b); v(x1, y + h); setC(a); v(x0, y + h);
      }
    }
  }
  // 段数随屏上半径变（逻辑半径 × 当前缩放），小圆少段
  // play-fighter：按物理像素算段数（× dpr），屏幕按键那种 30px 的圆在 3 倍屏上 14 段会看出棱角
  function segs(r) { var sc = Math.sqrt(m0 * m0 + m1 * m1); var s = Math.ceil(Math.sqrt(Math.abs(r) * sc * dpr) * 2.6); return s < 8 ? 8 : s > 64 ? 64 : s; }
  // 椭圆扇面（起止角，角度增大 = 顺时针，与 Canvas 一致：y 向下）
  function ellipse(cx, cy, rx, ry, rot, c, a0, a1) {
    setC(col(c));
    var full = a0 === undefined, k = segs(rx > ry ? rx : ry);
    if (!full) k = Math.max(2, Math.ceil(k * (a1 - a0) / TAU));
    var t = full ? unitCircle(k) : null, cr0 = Math.cos(rot || 0), sr0 = Math.sin(rot || 0);
    room(k * 3);
    var px = 0, py = 0;
    for (var i = 0; i <= k; i++) {
      var ct, st;
      if (full) { ct = t[i * 2]; st = t[i * 2 + 1]; } else { var a = a0 + (a1 - a0) * i / k; ct = Math.cos(a); st = Math.sin(a); }
      var ex = ct * rx, ey = st * ry, x = cx + ex * cr0 - ey * sr0, y = cy + ex * sr0 + ey * cr0;
      if (i) { v(cx, cy); v(px, py); v(x, y); }
      px = x; py = y;
    }
  }
  function circle(cx, cy, r, c) { ellipse(cx, cy, r, r, 0, c); }
  // 径向渐变的圆（中心 c、边缘透明）：扇形的中心顶点给颜色、外圈顶点给 0，GPU 插值出平滑衰减。光晕用它，一个圆就够，不用叠很多层
  function glow(cx, cy, r, c, inner) {
    c = col(c); setC(c);
    var k = segs(r), t = unitCircle(k), r0 = cr, g0 = cg, b0 = cb, a0 = ca;
    room(k * 3);
    for (var i = 0; i < k; i++) {
      cr = r0; cg = g0; cb = b0; ca = a0; v(cx, cy);
      cr = cg = cb = ca = 0;
      v(cx + t[i * 2] * r, cy + t[i * 2 + 1] * r); v(cx + t[i * 2 + 2] * r, cy + t[i * 2 + 3] * r);
    }
  }
  // 椭圆环（描边）
  function strokeEllipse(cx, cy, rx, ry, rot, lw, c) {
    setC(col(c));
    var k = segs(rx > ry ? rx : ry), t = unitCircle(k), cr0 = Math.cos(rot || 0), sr0 = Math.sin(rot || 0), h = lw / 2;
    room(k * 6);
    var p0 = 0, p1 = 0, p2 = 0, p3 = 0;
    for (var i = 0; i <= k; i++) {
      var ct = t[i * 2], st = t[i * 2 + 1];
      var ox = ct * (rx + h), oy = st * (ry + h), ix = ct * (rx - h), iy = st * (ry - h);
      var q0 = cx + ox * cr0 - oy * sr0, q1 = cy + ox * sr0 + oy * cr0, q2 = cx + ix * cr0 - iy * sr0, q3 = cy + ix * sr0 + iy * cr0;
      if (i) { v(p0, p1); v(q0, q1); v(q2, q3); v(p0, p1); v(q2, q3); v(p2, p3); }
      p0 = q0; p1 = q1; p2 = q2; p3 = q3;
    }
  }
  function strokeCircle(cx, cy, r, lw, c) { strokeEllipse(cx, cy, r, r, 0, lw, c); }
  // 凸多边形（扇形三角化）；pts = [[x, y], …]
  function poly(pts, c) {
    setC(col(c)); room((pts.length - 2) * 3);
    for (var i = 1; i + 1 < pts.length; i++) { v(pts[0][0], pts[0][1]); v(pts[i][0], pts[i][1]); v(pts[i + 1][0], pts[i + 1][1]); }
  }
  // 圆角矩形：中间三块矩形 + 四个角的四分之一圆
  function rrect(x, y, w, h, r, c) {
    r = Math.max(0, Math.min(r, w / 2, h / 2));
    if (!r) return rect(x, y, w, h, c);
    c = col(c);
    rect(x + r, y, w - 2 * r, h, c); rect(x, y + r, r, h - 2 * r, c); rect(x + w - r, y + r, r, h - 2 * r, c);
    ellipse(x + r, y + r, r, r, 0, c, Math.PI, Math.PI * 1.5); ellipse(x + w - r, y + r, r, r, 0, c, Math.PI * 1.5, TAU);
    ellipse(x + w - r, y + h - r, r, r, 0, c, 0, Math.PI / 2); ellipse(x + r, y + h - r, r, r, 0, c, Math.PI / 2, Math.PI);
  }
  // 粗折线：每段一块四边形，round=true 时每个顶点补一个圆（等于 lineJoin / lineCap round）
  function line(pts, lw, c, round) {
    c = col(c); setC(c); var h = lw / 2;
    for (var i = 0; i + 1 < pts.length; i++) {
      var a = pts[i], b = pts[i + 1], dx = b[0] - a[0], dy = b[1] - a[1], l = Math.sqrt(dx * dx + dy * dy);
      if (l < 1e-6) continue;
      var nx = -dy / l * h, ny = dx / l * h;
      quad(a[0] + nx, a[1] + ny, b[0] + nx, b[1] + ny, b[0] - nx, b[1] - ny, a[0] - nx, a[1] - ny);
    }
    // 圆角只补在两端和真正拐弯的点上（直线上的点补圆看不出来，白花顶点）
    if (round) for (var j = 0; j < pts.length; j++) {
      if (j > 0 && j + 1 < pts.length) {
        var p0 = pts[j - 1], p1 = pts[j], p2 = pts[j + 1];
        if (Math.abs((p1[0] - p0[0]) * (p2[1] - p1[1]) - (p1[1] - p0[1]) * (p2[0] - p1[0])) < 1e-3) continue;
      }
      ellipse(pts[j][0], pts[j][1], h, h, 0, c);
    }
  }
  function strokeRect(x, y, w, h, lw, c) {
    var k = lw / 2;
    c = col(c);
    rect(x - k, y - k, w + lw, lw, c); rect(x - k, y + h - k, w + lw, lw, c);
    rect(x - k, y + k, lw, h - lw, c); rect(x + w - k, y + k, lw, h - lw, c);
  }

  // ───────── 文字：每个字符（字、字号、颜色、粗细、描边）→ 图集里的一块 ─────────
  var glyphs = {};
  function glyph(ch, size, color, weight, outline) {
    var sig = ch + '\u0001' + size + '\u0001' + color + '\u0001' + (weight || '') + '\u0001' + (outline ? outline.join(',') : '');
    var e = glyphs[sig];
    if (e) return e;
    var c = scratch.getContext('2d');
    var font = (weight || 'normal') + ' ' + Math.round(size * dpr) + 'px sans-serif';
    c.font = font;
    var ow = outline ? outline[1] : 0, adv = c.measureText(ch).width / dpr;
    var w = Math.ceil(adv + 4 + ow * 2), h = Math.ceil(size * 1.35 + 4 + ow * 2);
    var pw = Math.ceil(w * dpr), ph = Math.ceil(h * dpr);
    if (shelfX + pw + 2 > ATLAS) { shelfX = 0; shelfY += shelfH + 2; shelfH = 0; }
    if (shelfY + ph > ATLAS) {                       // 满了：先把攒着的画掉（它们引用旧内容），再整张重来
      if (!recording) flush();
      glyphs = {}; resetAtlas();
      if (shelfX + pw + 2 > ATLAS) { shelfX = 0; shelfY += shelfH + 2; shelfH = 0; }
    }
    if (ch.trim()) {                                 // 空格不用画、不用传
      scratch.width = pw; scratch.height = ph;       // 改尺寸会清空画布与状态
      c = scratch.getContext('2d');
      c.font = font; c.textAlign = 'center'; c.textBaseline = 'middle';
      var cx = pw / 2, cy = ph / 2 + dpr * 0.5;
      if (outline) {
        c.fillStyle = outline[0];
        var r = ow * dpr;
        for (var k = 0; k < 16; k++) { var a = k / 16 * TAU; c.fillText(ch, cx + Math.cos(a) * r, cy + Math.sin(a) * r); }
      }
      c.fillStyle = color; c.fillText(ch, cx, cy);
      if (curTex !== atlas) { flush(); curTex = atlas; }
      gl.bindTexture(gl.TEXTURE_2D, atlas);
      gl.texSubImage2D(gl.TEXTURE_2D, 0, shelfX, shelfY, gl.RGBA, gl.UNSIGNED_BYTE, scratch);
    }
    e = glyphs[sig] = { w: w, h: h, adv: adv, blank: !ch.trim(), u0: shelfX / ATLAS, v0: shelfY / ATLAS, u1: (shelfX + pw) / ATLAS, v1: (shelfY + ph) / ATLAS };
    shelfX += pw + 2; shelfH = Math.max(shelfH, ph);
    return e;
  }
  // align：'left' | 'center' | 'right'（默认居中）；y 是文字中线。返回包围盒（逻辑像素，未变换）
  function text(s, x, y, size, color, align, weight, outline) {
    s = String(s);
    if (typeof color !== 'string') color = 'rgba(' + color.map(function (q, i) { return i < 3 ? Math.round(q * 255) : q; }).join(',') + ')';
    var gs = [], total = 0;
    for (var i = 0; i < s.length; i++) { var e = glyph(s[i], size, color, weight, outline); gs.push(e); total += e.adv; }
    var pen = align === 'left' ? x : align === 'right' ? x - total : x - total / 2;
    var x0 = pen, hh = size * 0.675 + 2;
    if (curTex !== atlas) useTex(atlas);
    var sa = alpha; cr = cg = cb = sa; ca = additive ? 0 : sa;   // 字的颜色在纹理里：顶点色 = 白 × alpha
    for (var j = 0; j < gs.length; j++) {
      var g = gs[j];
      if (!g.blank) {
        var lx = pen + g.adv / 2 - g.w / 2, ly = y - g.h / 2;
        room(6);
        vt(lx, ly, g.u0, g.v0); vt(lx + g.w, ly, g.u1, g.v0); vt(lx + g.w, ly + g.h, g.u1, g.v1);
        vt(lx, ly, g.u0, g.v0); vt(lx + g.w, ly + g.h, g.u1, g.v1); vt(lx, ly + g.h, g.u0, g.v1);
      }
      pen += g.adv;
    }
    return { x: x0, y: y - hh, w: total, h: hh * 2 };
  }

  // ───────── 贴图 ─────────
  // img：wx.createImage 加载好的图。返回 { tex, w, h }；NPOT 图只能 CLAMP + 不带 mipmap（WebGL 1）
  function texture(img) {
    var t = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, t);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);
    gl.bindTexture(gl.TEXTURE_2D, curTex);
    return { tex: t, w: img.width, h: img.height };
  }
  // 把贴图 T 里 (sx, sy, sw, sh) 那块画到 (dx, dy, dw, dh)（当前变换下）；flip 左右镜像；
  // tint = [r, g, b, a]（0~1，默认白）：乘在贴图上；additive 时相加
  function image(T, sx, sy, sw, sh, dx, dy, dw, dh, flip, tint) {
    useTex(T.tex);
    var tr = 1, tg = 1, tb = 1, ta = 1;
    if (tint) { tr = tint[0]; tg = tint[1]; tb = tint[2]; ta = tint[3] === undefined ? 1 : tint[3]; }
    var a = ta * alpha; cr = tr * a; cg = tg * a; cb = tb * a; ca = additive ? 0 : a;
    var u0 = sx / T.w, v0 = sy / T.h, u1 = (sx + sw) / T.w, v1 = (sy + sh) / T.h;
    if (flip) { var tu = u0; u0 = u1; u1 = tu; }
    room(6);
    vt(dx, dy, u0, v0); vt(dx + dw, dy, u1, v0); vt(dx + dw, dy + dh, u1, v1);
    vt(dx, dy, u0, v0); vt(dx + dw, dy + dh, u1, v1); vt(dx, dy + dh, u0, v1);
  }

  // ───────── 图层 / 精灵：fn 里画的东西录一次进自己的顶点缓冲，sig 不变就不重录 ─────────
  // 画的时候顶点 × scale + (x, y)，颜色 × a（uniform，不改缓冲）：录在原点附近、到处摆，就是精灵。
  // 录的时候不带当前变换与 alpha（录成「原样」）；fn 里不能超过 MAX_VERTS 个顶点
  function record(L, fn) {
    var sm = [m0, m1, m2, m3, m4, m5, alpha];
    m0 = 1; m1 = 0; m2 = 0; m3 = 1; m4 = 0; m5 = 0; alpha = 1;
    recording = true; n = 0;
    try { fn(); } finally { recording = false; m0 = sm[0]; m1 = sm[1]; m2 = sm[2]; m3 = sm[3]; m4 = sm[4]; m5 = sm[5]; alpha = sm[6]; }
    gl.bindBuffer(gl.ARRAY_BUFFER, L.vbo); bound = null;   // 绑了但没设指针：下次 bindBuf 一定重绑
    gl.bufferData(gl.ARRAY_BUFFER, buf.subarray(0, n * FLOATS), gl.STATIC_DRAW);
    L.count = n; n = 0;
  }
  function sprite(key, sig, fn, x, y, sc, a) {
    useTex(atlas); flush();
    var L = layers[key];
    if (!L) L = layers[key] = { vbo: gl.createBuffer(), count: 0, sig: null };
    if (L.sig !== sig) { record(L, fn); L.sig = sig; }
    if (!L.count) return;
    bindBuf(L.vbo);
    // 当前变换只取平移（精灵不跟着旋转 / 斜切）；当前 alpha 照乘
    uniforms(m0 * (x || 0) + m2 * (y || 0) + m4, m1 * (x || 0) + m3 * (y || 0) + m5, sc === undefined ? 1 : sc, (a === undefined ? 1 : a) * alpha);
    gl.drawArrays(gl.TRIANGLES, 0, L.count);
    draws++;
  }
  // 图层 = 摆在 (dx, dy)、不缩放的精灵
  function layer(key, sig, fn, dx, dy) { sprite(key, sig, fn, dx || 0, dy || 0, 1, 1); }

  return {
    gl: gl,
    // 一帧开头：清屏（背景色）、复位变换与 alpha
    // opaque：这一帧会画满整屏（不透明的背景图），跳过 clear
    begin: function (bg, opaque) {
      draws = 0; verts = 0; n = 0;
      m0 = 1; m1 = 0; m2 = 0; m3 = 1; m4 = 0; m5 = 0; alpha = 1; stack.length = 0;
      var c = col(bg || '#000');
      gl.viewport(0, 0, canvas.width, canvas.height);
      // 跳过 clear 不为省 GPU：真机上 clear 是这一帧第一次碰帧缓冲，拿下一块缓冲的等待（2~5ms）会记在它头上，
      // 跳过它这段等待就挪到第一次 draw 上，总时间不变。留着它是为了不画满整屏的画面（加载、标题）没有残影
      if (!opaque) { gl.clearColor(c[0], c[1], c[2], 1); gl.clear(gl.COLOR_BUFFER_BIT); }
      gl.bindTexture(gl.TEXTURE_2D, atlas); curTex = atlas; additive = false;
    },
    end: function () { flush(); return draws; },
    verts: function () { return verts; },
    save: function () { stack.push([m0, m1, m2, m3, m4, m5, alpha]); },
    restore: function () { var s = stack.pop(); if (s) { m0 = s[0]; m1 = s[1]; m2 = s[2]; m3 = s[3]; m4 = s[4]; m5 = s[5]; alpha = s[6]; } },
    translate: function (x, y) { m4 += m0 * x + m2 * y; m5 += m1 * x + m3 * y; },
    rotate: function (a) { var c = Math.cos(a), s = Math.sin(a), a0 = m0, a1 = m1, a2 = m2, a3 = m3; m0 = a0 * c + a2 * s; m1 = a1 * c + a3 * s; m2 = a2 * c - a0 * s; m3 = a3 * c - a1 * s; },
    scale: function (x, y) { m0 *= x; m1 *= x; m2 *= y; m3 *= y; },
    setAlpha: function (a) { alpha = a; }, getAlpha: function () { return alpha; },
    setBlend: function (m) { additive = m === 'add'; },
    texture: texture, image: image,
    // 胶囊按钮：画出来并返回命中框（与 base.js 的 button 同一用法）
    button: function (s, cx, cy, w, h, bg, fg) {
      rrect(cx - w / 2, cy - h / 2, w, h, h / 2, bg || '#ffffff');
      text(s, cx, cy + 1, Math.round(h * 0.42), fg || '#333333', 'center', 'bold');
      return { x: cx - w / 2, y: cy - h / 2, w: w, h: h };
    },
    rect: rect, gradRect: gradRect, rrect: rrect, circle: circle, glow: glow, ellipse: ellipse, poly: poly, line: line,
    strokeRect: strokeRect, strokeCircle: strokeCircle, strokeEllipse: strokeEllipse, text: text, layer: layer, sprite: sprite, color: col
  };
}

module.exports = { make: make };
