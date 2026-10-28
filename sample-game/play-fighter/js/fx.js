// 特效：粒子（火花、光晕、冲击环、火星、尘土、冰晶、闪电）、飞行道具的样子、场景氛围（雨 / 落樱）。
// 坐标是世界坐标（y = 离地高度，向上为正），画的时候 render.js 已经把变换设成「世界 → 屏幕」，这里画 (x, -y)。
// 发光的东西一律走叠加混合（gl2d.setBlend('add')），叠在一起越叠越亮，这是格斗游戏火花「炸开」的感觉的来源。
// 粒子用对象池（固定上限 600），逐帧不分配：真机上 GC 一次就是一帧顿。
var MAX = 600;

function FX(fight) {
  this.f = fight;
  this.pool = [];
  for (var i = 0; i < MAX; i++) this.pool.push({ on: false });
  this.n = 0;
  this.amb = [];       // 场景氛围粒子（单独存，不和打击特效抢上限）
  this.t = 0;
}

FX.prototype.clear = function () { for (var i = 0; i < MAX; i++) this.pool[i].on = false; this.amb.length = 0; };

FX.prototype.add = function (kind, x, y, vx, vy, life, size, c, extra) {
  for (var k = 0; k < MAX; k++) {
    var i = (this.n + k) % MAX, p = this.pool[i];
    if (p.on) continue;
    this.n = i + 1;
    p.on = true; p.kind = kind; p.x = x; p.y = y; p.vx = vx; p.vy = vy; p.life = life; p.max = life; p.size = size;
    p.r = c[0]; p.g = c[1]; p.b = c[2]; p.a = c.length > 3 ? c[3] : 1; p.ang = extra || 0; p.spin = 0; p.grav = 0;
    return p;
  }
  return null;
};

var WHITE = [1, 1, 1], HOT = [1, 0.85, 0.45], GUARD = [0.55, 0.8, 1];
var R = Math.random;

// 打中：白芯 + 彩色光晕 + 放射状光条 + 冲击环 + 火星
FX.prototype.spark = function (x, y, size, col, dir) {
  var c = col || HOT, s = size || 1;
  this.add('glow', x, y, 0, 0, 10, 60 * s, c, 0);
  this.add('glow', x, y, 0, 0, 6, 28 * s, WHITE, 0);
  this.add('ring', x, y, 0, 0, 12, 18 * s, c, 0);
  var n = Math.round(9 + 7 * s);
  for (var i = 0; i < n; i++) {
    var a = R() * Math.PI * 2, sp = (6 + R() * 10) * s;
    var st = this.add('streak', x, y, Math.cos(a) * sp + dir * 3, Math.sin(a) * sp, 8 + R() * 6, (16 + R() * 26) * s, i % 3 ? c : WHITE, a);
  }
  for (var j = 0; j < 8 * s; j++) {
    var e = this.add('ember', x, y, (R() - 0.5) * 12 * s + dir * 4, R() * 9 * s, 22 + R() * 20, 2 + R() * 2.5, c, 0);
    if (e) e.grav = 0.5;
  }
};
FX.prototype.guard = function (x, y, dir) {
  this.add('glow', x, y, 0, 0, 9, 40, GUARD, 0);
  this.add('ring', x, y, 0, 0, 10, 14, GUARD, 0);
  for (var i = 0; i < 6; i++) { var a = (i / 6) * Math.PI * 2 + R() * 0.3; this.add('streak', x, y, Math.cos(a) * 7, Math.sin(a) * 7, 8, 18, WHITE, a); }
};
FX.prototype.dust = function (x, s) {
  for (var i = 0; i < 6 * s; i++) {
    var d = this.add('smoke', x + (R() - 0.5) * 50, 4, (R() - 0.5) * 5, 0.6 + R() * 1.2, 28 + R() * 14, 14 + R() * 12 * s, [0.6, 0.55, 0.5, 0.45], 0);
  }
};
FX.prototype.clash = function (x, y) { this.spark(x, y, 1.6, WHITE, 0); this.add('ring', x, y, 0, 0, 18, 40, HOT, 0); };
FX.prototype.pop = function (q) {
  var c = this.f.p[0] === q.owner ? q.owner.def.glow : q.owner.def.glow;
  this.add('glow', q.x, q.y, 0, 0, 14, q.r * 2, c, 0);
  this.add('ring', q.x, q.y, 0, 0, 16, q.r * 0.6, c, 0);
};
// 升龙：脚下一圈火焰 / 冰晶往上卷
FX.prototype.rise = function (f) {
  var c = f.def.glow;
  for (var i = 0; i < 18; i++) {
    var e = this.add('ember', f.x + (R() - 0.5) * 70, R() * 60, (R() - 0.5) * 3, 6 + R() * 8, 24 + R() * 16, 3 + R() * 4, i % 3 ? c : WHITE, 0);
    if (e) e.grav = -0.1;
  }
  this.add('ring', f.x, 10, 0, 0, 14, 30, c, 0);
};
FX.prototype.superAura = function (f) {
  var c = f.def.glow;
  for (var i = 0; i < 40; i++) {
    var a = R() * Math.PI * 2, r = 120 + R() * 80;
    var e = this.add('ember', f.x + Math.cos(a) * r, 130 + Math.sin(a) * r, -Math.cos(a) * 5, -Math.sin(a) * 5, 26 + R() * 10, 3 + R() * 3, i % 4 ? c : WHITE, 0);
  }
  this.add('ring', f.x, 130, 0, 0, 30, 30, c, 0);
};
// 飞行道具每帧掉的拖尾
FX.prototype.trail = function (q) {
  var c = q.owner.def.glow, back = q.vx > 0 ? -1 : 1, big = q.big ? 2 : 1;
  for (var i = 0; i < 2 * big; i++) {
    var e = this.add(q.kind === 'yan' ? 'ember' : 'shard', q.x + back * q.r * 0.6 + (R() - 0.5) * q.r * 0.4, q.y + (R() - 0.5) * q.r * 0.9,
      back * (2 + R() * 4), (R() - 0.3) * 2.5, 16 + R() * 12, (q.kind === 'yan' ? 3 : 5) + R() * 3 * big, R() < 0.3 ? WHITE : c, R() * 6);
    if (e) { e.spin = (R() - 0.5) * 0.4; e.grav = q.kind === 'yan' ? -0.12 : 0.15; }
  }
};

FX.prototype.step = function () {
  this.t++;
  for (var i = 0; i < MAX; i++) {
    var p = this.pool[i];
    if (!p.on) continue;
    if (--p.life <= 0) { p.on = false; continue; }
    p.x += p.vx; p.y += p.vy; p.vy -= p.grav; p.ang += p.spin;
    if (p.kind === 'streak') { p.vx *= 0.82; p.vy *= 0.82; }
    if (p.kind === 'smoke') { p.vx *= 0.94; p.size *= 1.02; }
    if (p.kind === 'ember' && p.y < 0) { p.y = 0; p.vy *= -0.3; p.vx *= 0.7; }
  }
};

FX.prototype.draw = function (g) {
  // 先画普通混合的（尘土），再统一切叠加画发光的：一次切换，不来回 flush
  for (var i = 0; i < MAX; i++) {
    var p = this.pool[i];
    if (!p.on || p.kind !== 'smoke') continue;
    var k = p.life / p.max;
    g.circle(p.x, -p.y, p.size, [p.r, p.g, p.b, p.a * k]);
  }
  g.setBlend('add');
  for (var j = 0; j < MAX; j++) {
    var q = this.pool[j];
    if (!q.on || q.kind === 'smoke') continue;
    var t = q.life / q.max, col = [q.r, q.g, q.b, q.a * t];
    if (q.kind === 'glow') {
      var rr = q.size * (1.25 - 0.4 * t);
      col[3] = q.a * t * 0.9; g.glow(q.x, -q.y, rr, col);
    } else if (q.kind === 'ring') {
      g.strokeCircle(q.x, -q.y, q.size * (1 + (1 - t) * 3.2), 4 * t + 1, col);
    } else if (q.kind === 'streak') {
      var a = q.ang, l = q.size * t, w = 3.2 * t + 0.6, ux = Math.cos(a), uy = Math.sin(a);
      var hx = q.x, hy = -q.y;
      g.poly([[hx + ux * l, hy - uy * l], [hx - uy * w, hy - ux * w], [hx - ux * l * 0.3, hy + uy * l * 0.3], [hx + uy * w, hy + ux * w]], col);
    } else if (q.kind === 'ember') {
      g.circle(q.x, -q.y, q.size * (0.5 + t * 0.5), col);
    } else if (q.kind === 'shard') {
      var s = q.size, c0 = Math.cos(q.ang), s0 = Math.sin(q.ang), x = q.x, y = -q.y;
      g.poly([[x + c0 * s * 1.6, y + s0 * s * 1.6], [x - s0 * s * 0.5, y + c0 * s * 0.5], [x - c0 * s, y - s0 * s], [x + s0 * s * 0.5, y - c0 * s * 0.5]], col);
    }
  }
  g.setBlend();
};

// ───────── 飞行道具 ─────────
FX.prototype.drawProj = function (g, q) {
  var c = q.owner.def.glow, x = q.x, y = -q.y, r = q.r, t = this.t, dir = q.vx > 0 ? 1 : -1;
  g.setBlend('add');
  g.glow(x, y, r * 2.4, [c[0], c[1], c[2], 0.55]);
  if (q.kind === 'yan') {
    // 火焰：往后拖的几条火舌（三角），长度随时间抖动
    for (var i = 0; i < 7; i++) {
      var ph = t * 0.5 + i * 1.7, off = (i - 3) * r * 0.22, len = r * (1.6 + 0.6 * Math.sin(ph)) * (1 - Math.abs(i - 3) * 0.12);
      g.poly([[x + dir * r * 0.55, y + off * 0.7], [x - dir * len, y + off * 1.3 + Math.sin(ph * 1.3) * r * 0.15], [x + dir * r * 0.1, y + off * 0.25 + r * 0.18]],
        [1, 0.35 + 0.15 * Math.sin(ph), 0.08, 0.55]);
    }
    g.glow(x, y, r * 1.1, [1, 0.55, 0.15, 0.9]);
    g.circle(x + dir * r * 0.12, y, r * 0.55, [1, 0.85, 0.4, 0.9]);
    g.circle(x + dir * r * 0.18, y, r * 0.3, [1, 1, 0.9, 1]);
  } else {
    // 冰电：冰蓝色芯 + 绕着跳的闪电（每帧随机折线）
    g.glow(x, y, r * 1.05, [0.35, 0.75, 1, 0.9]);
    g.circle(x, y, r * 0.5, [0.75, 0.95, 1, 0.9]);
    g.circle(x, y, r * 0.26, [1, 1, 1, 1]);
    for (var b = 0; b < 4; b++) {
      var a0 = R() * Math.PI * 2, pts = [], seg = 5;
      for (var k = 0; k <= seg; k++) {
        var a = a0 + k * 0.5, rr = r * (0.4 + k / seg * 0.9);
        pts.push([x + Math.cos(a) * rr + (R() - 0.5) * 10, y + Math.sin(a) * rr + (R() - 0.5) * 10]);
      }
      g.line(pts, 2.2, [0.7, 0.9, 1, 0.9]);
    }
    for (var s = 0; s < 3; s++) {
      var aa = t * 0.2 + s * 2.1, sx = x + Math.cos(aa) * r * 0.9, sy = y + Math.sin(aa) * r * 0.6;
      g.poly([[sx + 9, sy], [sx, sy - 4], [sx - 9, sy], [sx, sy + 4]], [0.85, 0.97, 1, 0.9]);
    }
  }
  if (q.big) {
    // 超必杀：外面再套一圈旋转的光环
    for (var n = 0; n < 2; n++) g.strokeEllipse(x, y, r * (1.1 + n * 0.25), r * (0.75 + n * 0.2), t * 0.15 * (n ? -1 : 1), 3, [c[0], c[1], c[2], 0.5]);
  }
  g.setBlend();
};

// ───────── 场景氛围：屏幕坐标（不随镜头），画在角色之后、HUD 之前 ─────────
FX.prototype.ambient = function (g, kind, W, H, camDx) {
  var amb = this.amb;
  if (kind === 'rain') {
    while (amb.length < 90) amb.push({ x: R() * W, y: R() * H, v: 14 + R() * 10, l: 10 + R() * 16 });
    for (var i = 0; i < amb.length; i++) {
      var d = amb[i];
      d.y += d.v * 0.6; d.x -= d.v * 0.12 + camDx;
      if (d.y > H) { d.y = -20; d.x = R() * (W + 60); }
      if (d.x < -20) d.x += W + 40; if (d.x > W + 20) d.x -= W + 40;
      g.line([[d.x, d.y], [d.x - d.l * 0.2, d.y + d.l]], 1, 'rgba(190,210,255,0.28)');
    }
  } else if (kind === 'petals') {
    while (amb.length < 36) amb.push({ x: R() * W, y: R() * H, v: 0.6 + R() * 0.9, s: 2.5 + R() * 3, ph: R() * 6 });
    for (var j = 0; j < amb.length; j++) {
      var p = amb[j];
      p.ph += 0.04; p.y += p.v; p.x += Math.sin(p.ph) * 0.8 - 0.4 - camDx;
      if (p.y > H + 10) { p.y = -10; p.x = R() * (W + 80); }
      if (p.x < -20) p.x += W + 40; if (p.x > W + 20) p.x -= W + 40;
      g.ellipse(p.x, p.y, p.s, p.s * (0.45 + 0.35 * Math.sin(p.ph * 2)), p.ph, 'rgba(255,190,215,0.85)');
    }
  }
};

module.exports = FX;
