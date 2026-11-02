// 世界：对象池（飞行中的水果 / 炸弹、切开的半个、果汁粒子、果渍、飘字）、抛射节奏、刀光线段与圆的相交判定、连切。
// 帧里不分配：所有对象启动时建好，用 on 标记占用。
var B = require('./base.js');
var F = require('./fruits.js');
var W = B.W, H = B.H, TAU = Math.PI * 2, G = 1000;        // 重力（逻辑像素 / 秒²）

function pool(n, mk) { var a = []; for (var i = 0; i < n; i++) { var o = mk(); o.on = false; a.push(o); } a.cur = 0; return a; }
function take(a) {
  for (var i = 0, n = a.length; i < n; i++) {
    var k = a.cur + i; if (k >= n) k -= n;
    if (!a[k].on) { a.cur = k + 1 >= n ? 0 : k + 1; a[k].on = true; return a[k]; }
  }
  return null;
}
function count(a) { var c = 0; for (var i = 0; i < a.length; i++) if (a[i].on) c++; return c; }
function clear(a) { for (var i = 0; i < a.length; i++) a[i].on = false; }

var fruits = pool(18, function () { return { k: 0, x: 0, y: 0, vx: 0, vy: 0, a: 0, va: 0, r: 20 }; });
var halves = pool(36, function () { return { k: 0, x: 0, y: 0, vx: 0, vy: 0, a: 0, va: 0 }; });
var drops = pool(420, function () { return { x: 0, y: 0, vx: 0, vy: 0, life: 0, max: 1, s: 1, c: [1, 1, 1, 1], r: 1, g: 1, b: 1 }; });
var splats = pool(12, function () { return { x: 0, y: 0, t: 0, s: 1, c: [1, 1, 1, 1] }; });
var pops = pool(10, function () { return { x: 0, y: 0, t: 0, s: '', c: '#ffffff', size: 18 }; });

var S = {
  state: 'ready', mode: 'classic', score: 0, best: { classic: 0, timed: 0 }, lives: 3, timeLeft: 60, elapsed: 0,
  throwT: 0, burstLeft: 0, bombLeft: 0, burstT: 0, cuts: 0, maxCombo: 0, bombsHit: 0, flashT: 0, shake: 0,
  clock: 0, overAt: 0, boomT: 0, newBest: false, lastTick: 0, combo: { n: 0, t: 0, x: 0, y: 0 },
  // 菜单上的两个水果：切开（或点）就开始对应模式
  menu: [{ k: 0, mode: 'classic', x: W * 0.28, y: H * 0.6, a: 0, on: true }, { k: 1, mode: 'timed', x: W * 0.72, y: H * 0.6, a: 0, on: true }],
  menuGo: '', menuT: 0
};
var fx = function () {};

function reset(mode) {
  [fruits, halves, drops, pops].forEach(clear);
  S.state = 'play'; S.mode = mode; S.score = 0; S.lives = 3; S.timeLeft = 60; S.elapsed = 0; S.throwT = 0.6;
  S.burstLeft = 0; S.bombLeft = 0; S.cuts = 0; S.maxCombo = 0; S.bombsHit = 0; S.flashT = 0; S.shake = 0; S.newBest = false;
  S.combo.n = 0; S.combo.t = 0; S.lastTick = 0;
  fx('start');
}
function toMenu() {
  S.state = 'ready'; S.menuGo = '';
  for (var i = 0; i < S.menu.length; i++) S.menu[i].on = true;
}

// ───────── 抛射 ─────────
function throwOne(bomb) {
  var f = take(fruits); if (!f) return;
  f.k = bomb ? -1 : Math.floor(Math.random() * F.TYPES.length);
  f.r = bomb ? F.BOMB_R : F.TYPES[f.k].r;
  f.x = B.rand(50, W - 50); f.y = H + f.r;
  var apex = B.rand(H * 0.16, H * 0.45);
  f.vy = -Math.sqrt(2 * G * (f.y - apex));
  var tUp = -f.vy / G;
  f.vx = (B.rand(40, W - 40) - f.x) / (tUp * 2);          // 落回原高度时还在屏内
  f.a = Math.random() * TAU; f.va = B.rand(-4, 4);
  fx('throw');
}
function planBurst() {
  var e = S.elapsed, n, bombs = 0;
  if (S.mode === 'classic') {
    S.throwT = Math.max(0.9, 2.0 - e * 0.025);
    n = 1 + Math.floor(Math.random() * Math.min(4, 1 + e / 15));
    if (e > 6 && Math.random() < Math.min(0.45, 0.15 + e * 0.005)) bombs = e > 45 && Math.random() < 0.3 ? 2 : 1;
  } else {
    S.throwT = Math.max(0.8, 1.3 - e * 0.01);
    n = 2 + Math.floor(Math.random() * 3);
    if (Math.random() < 0.3) bombs = 1;
  }
  S.burstLeft = n + bombs; S.bombLeft = bombs; S.burstT = 0;
}

// ───────── 切 ─────────
// 线段 (x0,y0)-(x1,y1) 与圆 (cx,cy,r) 相交：圆心到线段的最近距离 ≤ r
function segHit(x0, y0, x1, y1, cx, cy, r) {
  var dx = x1 - x0, dy = y1 - y0, l2 = dx * dx + dy * dy;
  var t = l2 > 0 ? ((cx - x0) * dx + (cy - y0) * dy) / l2 : 0;
  t = t < 0 ? 0 : t > 1 ? 1 : t;
  var px = x0 + dx * t - cx, py = y0 + dy * t - cy;
  return px * px + py * py <= r * r;
}
function splitFruit(k, x, y, vx, vy, ux, uy) {
  var nx = -uy, ny = ux, base = Math.atan2(-nx, ny);    // 半个的局部 +y 转到法线方向
  for (var s = 0; s < 2; s++) {
    var h = take(halves); if (!h) break;
    var sg = s ? -1 : 1;
    h.k = k; h.x = x + nx * sg * 3; h.y = y + ny * sg * 3;
    h.vx = vx * 0.5 + nx * sg * 110 + ux * 40; h.vy = Math.min(vy * 0.5, 0) + ny * sg * 110 - 60;
    h.a = base + (s ? Math.PI : 0); h.va = sg * B.rand(2, 4);
  }
  var T = F.TYPES[k], j = T.juice;
  for (var i = 0; i < 16; i++) {
    var p = take(drops); if (!p) break;
    var a = Math.random() * TAU, sp = B.rand(80, 320);
    p.x = x; p.y = y; p.vx = Math.cos(a) * sp + ux * 120; p.vy = Math.sin(a) * sp + uy * 120 - 80;
    p.life = p.max = B.rand(0.45, 0.85); p.s = B.rand(0.25, 0.6); p.r = j[0]; p.g = j[1]; p.b = j[2];
  }
  var sp2 = take(splats);
  if (sp2) { sp2.x = x; sp2.y = y; sp2.t = 0; sp2.s = B.rand(1.3, 1.9) * T.r / 30; sp2.c[0] = j[0]; sp2.c[1] = j[1]; sp2.c[2] = j[2]; }
}
function pop(x, y, s, c, size) { var p = take(pops); if (p) { p.x = x; p.y = y; p.t = 0; p.s = s; p.c = c; p.size = size || 18; } }
function cut(f, ux, uy) {
  f.on = false;
  splitFruit(f.k, f.x, f.y, f.vx, f.vy, ux, uy);
  S.score += 1; S.cuts++;
  var c = S.combo;
  c.n = c.t > 0 ? c.n + 1 : 1; c.t = 0.28; c.x = f.x; c.y = f.y;
  fx('cut', c.n);
}
function hitBomb(f) {
  f.on = false; S.bombsHit++;
  for (var i = 0; i < 40; i++) {
    var p = take(drops); if (!p) break;
    var a = Math.random() * TAU, sp = B.rand(100, 480), g = Math.random() < 0.5;
    p.x = f.x; p.y = f.y; p.vx = Math.cos(a) * sp; p.vy = Math.sin(a) * sp;
    p.life = p.max = B.rand(0.4, 0.9); p.s = B.rand(0.4, 0.9);
    p.r = g ? 0.35 : 1; p.g = g ? 0.35 : 0.6; p.b = g ? 0.35 : 0.15;
  }
  S.shake = 14;
  try { wx.vibrateLong && wx.vibrateLong(); } catch (e) {}
  if (S.mode === 'classic') { S.state = 'boom'; S.boomT = 1.0; S.flashT = 1.0; fx('bomb'); }
  else { S.score = Math.max(0, S.score - 10); S.flashT = 0.35; pop(f.x, f.y, '-10', '#ff5252', 26); fx('bombSmall'); }
}
// 刀光的一段：返回这一段切开了几个
function slice(x0, y0, x1, y1) {
  var dx = x1 - x0, dy = y1 - y0, l = Math.sqrt(dx * dx + dy * dy);
  if (l < 2) return 0;
  var ux = dx / l, uy = dy / l, n = 0, i;
  if (S.state === 'ready' && !S.menuGo) {
    for (i = 0; i < S.menu.length; i++) {
      var m = S.menu[i];
      if (m.on && segHit(x0, y0, x1, y1, m.x, m.y, F.TYPES[m.k].r + 6)) {
        m.on = false; splitFruit(m.k, m.x, m.y, 0, 0, ux, uy); fx('cut', 1);
        S.menuGo = m.mode; S.menuT = 0.45; return 1;
      }
    }
    return 0;
  }
  if (S.state !== 'play') return 0;
  for (i = 0; i < fruits.length; i++) {
    var f = fruits[i];
    if (!f.on || !segHit(x0, y0, x1, y1, f.x, f.y, f.r + 2)) continue;
    if (f.k < 0) { hitBomb(f); if (S.state !== 'play') return n; } else { cut(f, ux, uy); n++; }
  }
  return n;
}
function over() {
  S.state = 'over'; S.overAt = S.clock;
  var b = S.best[S.mode];
  if (S.score > b) { S.best[S.mode] = S.score; S.newBest = true; B.save('fruit.best.' + S.mode, S.score); }
  fx('over');
}

// ───────── 每帧 ─────────
function physics(dt) {
  var i;
  for (i = 0; i < halves.length; i++) {
    var h = halves[i]; if (!h.on) continue;
    h.vy += G * dt; h.x += h.vx * dt; h.y += h.vy * dt; h.a += h.va * dt;
    if (h.y > H + 60) h.on = false;
  }
  for (i = 0; i < drops.length; i++) {
    var p = drops[i]; if (!p.on) continue;
    p.life -= dt; if (p.life <= 0) { p.on = false; continue; }
    p.vy += 700 * dt; p.x += p.vx * dt; p.y += p.vy * dt;
  }
  for (i = 0; i < splats.length; i++) { var s = splats[i]; if (s.on && (s.t += dt) > 3) s.on = false; }
  for (i = 0; i < pops.length; i++) { var q = pops[i]; if (q.on) { q.t += dt; q.y -= 30 * dt; if (q.t > 1) q.on = false; } }
}
function fly(dt) {
  for (var i = 0; i < fruits.length; i++) {
    var f = fruits[i]; if (!f.on) continue;
    f.vy += G * dt; f.x += f.vx * dt; f.y += f.vy * dt; f.a += f.va * dt;
    if (f.vy > 0 && f.y > H + f.r + 10) {
      f.on = false;
      if (f.k >= 0 && S.state === 'play' && S.mode === 'classic') {   // 经典模式漏掉一个水果扣一条命
        S.lives--; pop(B.clamp(f.x, 30, W - 30), H - 40, '×', '#e53935', 34); fx('miss');
        if (S.lives <= 0) over();
      }
    }
  }
}
function update(dt) {
  S.clock += dt;
  if (S.shake > 0) S.shake = Math.max(0, S.shake - dt * 30);
  if (S.flashT > 0) S.flashT -= dt;
  physics(dt);
  if (S.state === 'ready') {
    for (var m = 0; m < S.menu.length; m++) S.menu[m].a += dt * (m ? -0.8 : 0.8);
    if (S.menuGo && (S.menuT -= dt) <= 0) reset(S.menuGo);
    return;
  }
  fly(dt);
  if (S.state === 'boom') { if ((S.boomT -= dt) <= 0) over(); return; }
  if (S.state !== 'play') return;
  S.elapsed += dt;
  if (S.mode === 'timed') {
    S.timeLeft -= dt;
    var sec = Math.ceil(S.timeLeft);
    if (sec <= 5 && sec > 0 && sec !== S.lastTick) { S.lastTick = sec; fx('tick'); }
    if (S.timeLeft <= 0) { S.timeLeft = 0; over(); return; }
  }
  if ((S.throwT -= dt) <= 0) planBurst();
  if (S.burstLeft > 0 && (S.burstT -= dt) <= 0) {
    var bomb = S.bombLeft > 0 && Math.random() < S.bombLeft / S.burstLeft;
    if (bomb) S.bombLeft--;
    S.burstLeft--; S.burstT = 0.13;
    throwOne(bomb);
  }
  // 连切：0.28 秒内接着切到的算一串，断了结算（≥3 个奖励个数分）
  var c = S.combo;
  if (c.t > 0 && (c.t -= dt) <= 0) {
    if (c.n >= 3) { S.score += c.n; pop(c.x, c.y - 30, c.n + ' 连切 +' + c.n, '#ffd54f', 24); fx('combo', c.n); }
    if (c.n > S.maxCombo) S.maxCombo = c.n;
    c.n = 0;
  }
}

module.exports = {
  S: S, fruits: fruits, halves: halves, drops: drops, splats: splats, pops: pops,
  count: count, reset: reset, toMenu: toMenu, slice: slice, update: update, over: over, segHit: segHit,
  setFx: function (f) { fx = f; }
};
