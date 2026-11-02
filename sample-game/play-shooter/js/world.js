// 世界：对象池（子弹 / 敌机 / 道具 / 粒子 / 冲击波 / 飘字 / 出场表）、波次、首领、碰撞。
// 帧里不分配：所有对象启动时一次建好，用 on 标记占用，take() 从游标往后找空位（满了就不生成：粒子少几颗看不出来）。
var B = require('./base.js');
var W = B.W, H = B.H, TAU = Math.PI * 2;

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

var pb = pool(160, function () { return { x: 0, y: 0, vx: 0, vy: 0 }; });
var eb = pool(420, function () { return { x: 0, y: 0, vx: 0, vy: 0, r: 4, k: 0 }; });
var en = pool(40, function () { return { type: 0, x: 0, y: 0, vx: 0, vy: 0, hp: 0, r: 0, t: 0, fire: 0, flash: 0, mode: 0, x0: 0, amp: 0, freq: 0, ty: 0, stay: 0, shot: 0 }; });
var pu = pool(10, function () { return { x: 0, y: 0, vx: 0, vy: 0, k: 0, t: 0 }; });
var pt = pool(800, function () { return { x: 0, y: 0, vx: 0, vy: 0, life: 0, max: 1, s: 1, drag: 0, c: [1, 1, 1, 1], r: 1, g: 1, b: 1 }; });
var rings = pool(12, function () { return { x: 0, y: 0, t: 0, max: 1, r1: 40, c: [1, 1, 1, 1] }; });
var texts = pool(16, function () { return { x: 0, y: 0, t: 0, s: '', c: '#ffffff' }; });
var sched = pool(64, function () { return { t: 0, type: 0, x: 0, mode: 0 }; });

// 三种敌机：半径、血量（随波次加）、分数、掉道具概率
var TYPES = [
  { r: 14, hp: 1, hpw: 1 / 6, score: 100, drop: 0.05, size: 0 },    // 侦察机：快、成群，第 4 波起偶尔放一枪
  { r: 19, hp: 5, hpw: 1 / 3, score: 300, drop: 0.14, size: 1 },    // 战斗机：飞到半空悬停，三连发瞄准
  { r: 30, hp: 16, hpw: 1, score: 800, drop: 0.4, size: 2 }         // 轰炸机：慢、硬，十向环形弹
];
var FIRE = [[1, 0.9, 0.5], [1, 0.55, 0.15], [1, 0.3, 0.25]];
var PLASMA = [[0.5, 0.9, 1], [0.8, 0.95, 1], [0.4, 0.6, 1]];
var PUNAME = ['火力升级', '护盾 8 秒', '炸弹 +1'];

var S = {
  state: 'ready', score: 0, best: 0, wave: 0, lives: 3, bombs: 1, power: 1, powerT: 0, shieldT: 0, invT: 0,
  kills: 0, sx: W / 2, sy: H - 120, fireT: 0, waveT: 0, banner: '', bannerT: 0, nextWaveAt: 0, bossPending: false,
  flashT: 0, shake: 0, clock: 0, overAt: 0, newBest: false, scroll: 0, ebSpeed: 1, shots: 0,
  boss: { on: false, x: 0, y: 0, hp: 0, max: 1, t: 0, mt: 0, pat: 0, patT: 0, fire: 0, ang: 0, burst: 0, flash: 0, tier: 1 }
};
var fx = function () {};

function reset(w0) {
  [pb, eb, en, pu, pt, rings, texts, sched].forEach(clear);
  S.state = 'play'; S.score = 0; S.lives = 3; S.bombs = 1; S.power = 1; S.powerT = 0; S.shieldT = 0; S.invT = 1.5;
  S.kills = 0; S.sx = W / 2; S.sy = H - 120; S.fireT = 0; S.flashT = 0; S.shake = 0; S.newBest = false; S.shots = 0;
  S.boss.on = false;
  startWave(w0 || 1);
}
function startWave(n) {
  S.wave = n; S.waveT = 0; S.nextWaveAt = 0; S.bossPending = false;
  S.ebSpeed = Math.min(1.6, 1 + (n - 1) * 0.05);
  clear(sched);
  if (n % 5 === 0) { S.banner = '警告：首领来袭'; S.bannerT = 2.4; S.bossPending = true; fx('warn'); return; }
  S.banner = '第 ' + n + ' 波'; S.bannerT = 1.6;
  var groups = Math.min(10, 3 + n), t = 1.2, gap = 1.7 - Math.min(0.7, n * 0.06);
  for (var g = 0; g < groups; g++, t += gap) {
    var r = Math.random();
    if (n >= 3 && r < 0.16) plan(t, 2, B.rand(70, W - 70), 0);
    else if (n >= 2 && r < 0.5) { plan(t, 1, W * 0.3, 0); plan(t + 0.3, 1, W * 0.7, 0); }
    else if (r < 0.75) {                          // 一串蛇形侦察机
      var x0 = B.rand(90, W - 90);
      for (var i = 0; i < 5; i++) plan(t + i * 0.22, 0, x0, 1);
    } else {                                      // V 字俯冲
      var cx = B.rand(100, W - 100);
      for (var j = 0; j < 5; j++) plan(t + Math.abs(j - 2) * 0.18, 0, cx + (j - 2) * 34, 0);
    }
  }
}
function plan(t, type, x, mode) { var s = take(sched); if (s) { s.t = t; s.type = type; s.x = x; s.mode = mode; } }

function spawn(type, x, mode) {
  var e = take(en); if (!e) return;
  var T = TYPES[type];
  e.type = type; e.x = x; e.x0 = x; e.y = -T.r - 10; e.r = T.r; e.t = 0; e.flash = -1; e.mode = mode; e.shot = 0;
  e.hp = Math.round(T.hp + (S.wave - 1) * T.hpw);
  if (type === 0) {
    e.vy = 170 + S.wave * 5; e.amp = 60; e.freq = 2.4;
    e.vx = mode === 0 ? (S.sx - x) * 0.18 : 0;
  } else if (type === 1) { e.ty = B.rand(110, 250); e.fire = 1.2; e.stay = 7; e.vx = 0; e.vy = 0; }
  else { e.vy = 38; e.fire = 1.5; e.vx = 0; }
}
function fire(x, y, ang, sp, k) {
  var b = take(eb); if (!b) return;
  sp *= S.ebSpeed;
  b.x = x; b.y = y; b.vx = Math.cos(ang) * sp; b.vy = Math.sin(ang) * sp; b.k = k; b.r = k === 1 ? 6 : 4;
}
function aim(x, y) { return Math.atan2(S.sy - y, S.sx - x); }

// ───────── 特效 ─────────
function burst(x, y, size, pal) {
  var n = [14, 22, 36, 90][size];
  for (var i = 0; i < n; i++) {
    var p = take(pt); if (!p) break;
    var a = Math.random() * TAU, sp = B.rand(40, 220) * (1 + size * 0.35), c = pal[i % 3];
    p.x = x; p.y = y; p.vx = Math.cos(a) * sp; p.vy = Math.sin(a) * sp;
    p.life = p.max = B.rand(0.35, 0.8) * (1 + size * 0.25); p.s = B.rand(0.5, 1.2) * (1 + size * 0.25); p.drag = 2.4;
    p.r = c[0]; p.g = c[1]; p.b = c[2];
  }
  var g = take(rings);
  if (g) { g.x = x; g.y = y; g.t = 0; g.max = 0.4 + size * 0.12; g.r1 = 26 + size * 26; g.c[0] = pal[0][0]; g.c[1] = pal[0][1]; g.c[2] = pal[0][2]; }
  if (size >= 2) S.shake = Math.max(S.shake, size === 3 ? 14 : 6);
}
function spark(x, y, pal) {
  var p = take(pt); if (!p) return;
  var a = Math.random() * TAU, sp = B.rand(60, 160), c = pal[1];
  p.x = x; p.y = y; p.vx = Math.cos(a) * sp; p.vy = Math.sin(a) * sp - 40; p.life = p.max = 0.22; p.s = 0.45; p.drag = 3;
  p.r = c[0]; p.g = c[1]; p.b = c[2];
}
function popText(x, y, s, c) { var t = take(texts); if (t) { t.x = x; t.y = y; t.t = 0; t.s = s; t.c = c; } }

// ───────── 得分、掉落、受伤 ─────────
function gain(base, x, y) {
  var v = Math.round(base * (1 + (S.wave - 1) * 0.1) / 10) * 10;
  S.score += v; popText(x, y, '+' + v, '#fff59d');
}
function kill(e) {
  var T = TYPES[e.type];
  e.on = false; S.kills++;
  gain(T.score, e.x, e.y);
  burst(e.x, e.y, T.size, FIRE);
  fx(T.size >= 2 ? 'boomBig' : 'boom');
  if (Math.random() < T.drop) drop(e.x, e.y);
}
function drop(x, y) {
  var p = take(pu); if (!p) return;
  var r = Math.random();
  p.k = r < 0.45 ? 0 : r < 0.75 ? 1 : 2;
  p.x = x; p.y = y; p.vx = B.rand(-40, 40); p.vy = 70; p.t = 0;
}
// 受击闪白：亮 0.05 秒、之后至少 0.08 秒不再闪（flash 一直往下减到 -1）。每一发都闪的话持续开火时敌机一半时间是白的
function damage(e, d) { e.hp -= d; if (e.flash < -0.08) e.flash = 0.05; if (e.hp <= 0) kill(e); }
function bossDamage(d) {
  var b = S.boss; b.hp -= d; if (b.flash < -0.08) b.flash = 0.05;
  if (b.hp > 0) return;
  b.on = false; S.kills++;
  gain(5000 * b.tier, b.x, b.y);
  burst(b.x, b.y, 3, FIRE); burst(b.x - 50, b.y, 2, FIRE); burst(b.x + 50, b.y + 10, 2, FIRE); burst(b.x, b.y - 20, 2, PLASMA);
  for (var i = 0; i < eb.length; i++) if (eb[i].on) { eb[i].on = false; if (i % 3 === 0) spark(eb[i].x, eb[i].y, PLASMA); }
  drop(b.x - 30, b.y); drop(b.x + 30, b.y);
  fx('bossDown'); B.buzz();
}
function hurt() {
  if (S.invT > 0 || S.state !== 'play') return;
  S.lives--; S.invT = 2.4; S.power = Math.max(1, S.power - 1); S.powerT = S.power > 1 ? 10 : 0;
  burst(S.sx, S.sy, 2, PLASMA); fx('hurt'); B.buzz();
  // 附近的子弹清掉，给玩家喘口气
  for (var i = 0; i < eb.length; i++) {
    var b = eb[i]; if (!b.on) continue;
    var dx = b.x - S.sx, dy = b.y - S.sy;
    if (dx * dx + dy * dy < 140 * 140) { b.on = false; spark(b.x, b.y, PLASMA); }
  }
  if (S.lives <= 0) over();
}
function over() {
  S.state = 'over'; S.overAt = S.clock; S.boss.on = false;
  burst(S.sx, S.sy, 3, PLASMA); fx('over');
  if (S.score > S.best) { S.best = S.score; S.newBest = true; B.save('shooter.best', S.best); }
}
function collect(p) {
  p.on = false; fx('power');
  if (p.k === 0) { S.power = Math.min(3, S.power + 1); S.powerT = 14; }
  else if (p.k === 1) S.shieldT = 8;
  else if (S.bombs < 3) S.bombs++;
  else { S.score += 500; }
  popText(S.sx, S.sy - 40, PUNAME[p.k], '#80deea');
}
// 炸弹：清屏上所有敌弹、所有敌机吃 30 点伤害、首领吃 8% 血
function bomb() {
  if (S.state !== 'play' || S.bombs <= 0) return false;
  S.bombs--; S.flashT = 0.5; S.shake = 12; S.invT = Math.max(S.invT, 1);
  for (var i = 0; i < eb.length; i++) if (eb[i].on) { eb[i].on = false; if (i % 2 === 0) spark(eb[i].x, eb[i].y, FIRE); }
  for (var j = 0; j < en.length; j++) if (en[j].on && en[j].y > -10) damage(en[j], 30);
  if (S.boss.on && S.boss.y > 0) bossDamage(Math.ceil(S.boss.max * 0.08));
  var g = take(rings);
  if (g) { g.x = S.sx; g.y = S.sy; g.t = 0; g.max = 0.6; g.r1 = H; g.c[0] = 1; g.c[1] = 0.9; g.c[2] = 0.6; }
  fx('bomb'); B.buzz();
  return true;
}

module.exports = {
  S: S, pb: pb, eb: eb, en: en, pu: pu, pt: pt, rings: rings, texts: texts, sched: sched, TYPES: TYPES,
  FIRE: FIRE, PLASMA: PLASMA,
  take: take, count: count, reset: reset, startWave: startWave, spawn: spawn, fire: fire, aim: aim,
  burst: burst, spark: spark, damage: damage, bossDamage: bossDamage, hurt: hurt, over: over, collect: collect, bomb: bomb,
  setFx: function (f) { fx = f; }
};
