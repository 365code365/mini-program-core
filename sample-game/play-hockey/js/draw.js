// 画面：冰面（裁剪清屏 + 不变的标线录成图层）、半场大比分、球的拖尾、球拍、碰撞光圈。其余全部走 gl2d 一批三角形。
var P = require('./physics.js');
var COLORS = { red: '#e53935', redD: '#b71c1c', redL: '#ff8a80', blue: '#1e88e5', blueD: '#0d47a1', blueL: '#82b1ff' };

function make(R, rink, SH) {
  var trail = new Float32Array(16), tn = 0, ti = 0, trailT = 0;     // 最近 8 个位置（环形）
  var rings = [];                                                   // 碰撞光圈（对象池）
  for (var i = 0; i < 8; i++) rings.push({ t: 0, x: 0, y: 0, c: '#ffffff' });

  // 护栏与冰面：看上去是三层叠着的圆角矩形。为什么不直接画三个整块 rrect：无头 CPU 后端（swiftshader + 4x MSAA）
  // 一帧的大头是填充面积，三层整块圆角矩形 + 一层反光渐变一帧 20~33ms，现在的写法 8~9ms（同一进程逐帧交替量，
  // gl.finish 计时）；真机 GPU 上两种写法一样快。现在：整帧清屏色就是冰面色（game.js 的 R.begin），
  // 外面每层只是一圈「框」，拆成上下左右四条裁剪清屏（窄条，便宜）；四个圆角靠图层里录的「切角」补出来。
  // LAYERS：[外扩, 圆角半径, 颜色]，由外到内；最外面是背景色 BG
  var BG = '#102027', ICE = '#eef6fb';
  var LAYERS = [[12, 30, '#455a64'], [6, 26, '#cfd8dc'], [0, 22, ICE]];
  function clears() {
    var gl = R.gl, bw = gl.drawingBufferWidth, bh = gl.drawingBufferHeight, k = bh / SH;
    gl.enable(gl.SCISSOR_TEST);
    var ox0 = 0, oy0 = 0, ox1 = bw, oy1 = bh;                       // 外框（设备像素，y 向下）
    for (var i = 0; i < LAYERS.length; i++) {
      var m = LAYERS[i][0], c = R.color(i ? LAYERS[i - 1][2] : BG);
      var x0 = Math.round((rink.L - m) * k), x1 = Math.round((rink.R + m) * k), y0 = Math.round((rink.T - m) * k), y1 = Math.round((rink.B + m) * k);
      gl.clearColor(c[0], c[1], c[2], 1);
      strip(gl, bh, ox0, oy0, ox1, y0); strip(gl, bh, ox0, y1, ox1, oy1);
      strip(gl, bh, ox0, y0, x0, y1); strip(gl, bh, x1, y0, ox1, y1);
      ox0 = x0; oy0 = y0; ox1 = x1; oy1 = y1;
    }
    gl.disable(gl.SCISSOR_TEST);
  }
  function strip(gl, bh, x0, y0, x1, y1) {
    if (x1 <= x0 || y1 <= y0) return;
    gl.scissor(x0, bh - y1, x1 - x0, y1 - y0); gl.clear(gl.COLOR_BUFFER_BIT);
  }
  // 切角：方角里圆弧以外的那块涂成外面一层的颜色 = 圆角。从方角顶点向圆弧扇形三角化（这块区域从顶点看是星形的）
  function cornerMask(cx, cy, sx, sy, r, color) {
    var n = 10, px = cx + sx * r, py = cy + sy * r;
    for (var i = 0; i < n; i++) {
      var a0 = Math.PI / 2 * i / n, a1 = Math.PI / 2 * (i + 1) / n;
      R.quad(cx, cy, px - sx * r * Math.cos(a0), py - sy * r * Math.sin(a0),
        px - sx * r * Math.cos(a1), py - sy * r * Math.sin(a1), cx, cy, color);
    }
  }
  function rinkShape() {
    var L = rink.L, T = rink.T, W = rink.W, H = rink.H, cy = rink.cy, mid = rink.mid, g = rink.goalH;
    // 切角由内到外画：一个点最后落的是「包含它的最内一层」的颜色，与三层圆角矩形叠着画一样
    for (var i = LAYERS.length - 1; i >= 0; i--) {
      var m = LAYERS[i][0], r = LAYERS[i][1], c = i ? LAYERS[i - 1][2] : BG, x0 = L - m, x1 = rink.R + m, y0 = T - m, y1 = rink.B + m;
      cornerMask(x0, y0, 1, 1, r, c); cornerMask(x1, y0, -1, 1, r, c);
      cornerMask(x0, y1, 1, -1, r, c); cornerMask(x1, y1, -1, -1, r, c);
    }
    // 球门：护栏上开口 + 门洞
    R.rect(L - 14, cy - g / 2, 16, g, '#263238');
    R.rect(rink.R - 2, cy - g / 2, 16, g, '#263238');
    R.rect(L - 3, cy - g / 2 - 3, 3, g + 6, COLORS.red);
    R.rect(rink.R, cy - g / 2 - 3, 3, g + 6, COLORS.blue);
    // 中线 + 中圈 + 开球点
    R.rect(mid - 2, T, 4, H, 'rgba(229,57,53,0.55)');
    R.strokeCircle(mid, cy, H * 0.18, 3, 'rgba(229,57,53,0.55)');
    R.circle(mid, cy, 6, 'rgba(229,57,53,0.7)');
    // 两边的禁区半圆（门前）与蓝线
    R.ellipse(L, cy, g * 0.62, g * 0.62, 0, 'rgba(229,57,53,0.16)', -Math.PI / 2, Math.PI / 2);
    R.ellipse(rink.R, cy, g * 0.62, g * 0.62, 0, 'rgba(30,136,229,0.16)', Math.PI / 2, Math.PI * 1.5);
    R.rect(L + W * 0.25 - 2, T, 4, H, 'rgba(30,136,229,0.35)');
    R.rect(L + W * 0.75 - 2, T, 4, H, 'rgba(30,136,229,0.35)');
    [0.25, 0.75].forEach(function (fx) {
      [0.3, 0.7].forEach(function (fy) { R.strokeCircle(L + W * fx, T + H * fy, 14, 2, 'rgba(229,57,53,0.35)'); });
    });
  }

  function mallet(m, color, dark, light, held) {
    var r = P.MALLET_R;
    R.circle(m.x + 2, m.y + 4, r + 1, 'rgba(0,0,0,0.18)');
    if (held) R.circle(m.x, m.y, r + 6, 'rgba(255,255,255,0.55)');
    R.circle(m.x, m.y, r, dark);
    R.circle(m.x, m.y, r * 0.82, color);
    R.circle(m.x, m.y, r * 0.42, dark);
    R.circle(m.x - 2, m.y - 2, r * 0.32, light);
  }

  function frame(world, scores, clock, dt) {
    clears();                                         // 必须在这一帧还没攒顶点时调：清屏立即生效
    R.layer('rink', 1, rinkShape);
    // 半场大比分（压在冰面下层的水印）
    R.text(String(scores[0]), rink.L + rink.W * 0.25, rink.cy, Math.round(rink.H * 0.36), 'rgba(229,57,53,0.13)', 'center', 'bold');
    R.text(String(scores[1]), rink.L + rink.W * 0.75, rink.cy, Math.round(rink.H * 0.36), 'rgba(30,136,229,0.13)', 'center', 'bold');
    var p = world.puck;
    trailT += dt;
    if (trailT > 1 / 60) {
      trailT = 0;
      trail[ti * 2] = p.x; trail[ti * 2 + 1] = p.y; ti = (ti + 1) % 8; if (tn < 8) tn++;
    }
    var sp = Math.sqrt(p.vx * p.vx + p.vy * p.vy);
    if (sp > 300) for (var i = 0; i < tn; i++) {
      var j = (ti - 1 - i + 16) % 8, a = (1 - i / tn) * Math.min(1, (sp - 300) / 600) * 0.35;
      R.circle(trail[j * 2], trail[j * 2 + 1], P.PUCK_R * (1 - i * 0.08), 'rgba(38,50,56,' + a.toFixed(2) + ')');
    }
    for (i = 0; i < rings.length; i++) {
      var g = rings[i];
      if (g.t <= 0) continue;
      g.t -= dt;
      R.setAlpha(Math.max(0, g.t / 0.35));
      R.strokeCircle(g.x, g.y, P.PUCK_R + (0.35 - g.t) * 90, 3, g.c);
      R.setAlpha(1);
    }
    var ms = world.mallets;
    mallet(ms[0], COLORS.red, COLORS.redD, COLORS.redL, ms[0].touch !== null);
    mallet(ms[1], COLORS.blue, COLORS.blueD, COLORS.blueL, ms[1].touch !== null);
    R.circle(p.x + 2, p.y + 3, P.PUCK_R, 'rgba(0,0,0,0.2)');
    R.circle(p.x, p.y, P.PUCK_R, '#212121');
    R.strokeCircle(p.x, p.y, P.PUCK_R * 0.62, 2, '#616161');
  }
  function ring(x, y, c) {
    for (var i = 0; i < rings.length; i++) if (rings[i].t <= 0) { rings[i].t = 0.35; rings[i].x = x; rings[i].y = y; rings[i].c = c; return; }
  }
  function clearTrail() { tn = 0; }
  return { frame: frame, ring: ring, clearTrail: clearTrail, ice: ICE };
}

module.exports = { make: make, COLORS: COLORS };
