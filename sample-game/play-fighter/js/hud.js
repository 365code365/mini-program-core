// 对战 HUD：血条（斜切的金属框、黄橙渐变、白色掉血残影、残血闪红）、头像、计时器、回合胜标、能量槽、连击数、
// 必杀技喊招、回合播报（ROUND / FIGHT / K.O.）与超必杀的切入演出。全部屏幕坐标。
var B = require('./base.js');
var D = require('./data.js');
var Fight = require('./fight.js');

var W = B.W, H = B.H;
var L = (B.safe.left || 0) + 8, Rr = (B.safe.right || W) - 8;
var TOP = Math.min(Math.max(B.safe.top || 0, 4) + 8, 40);

// 斜切的平行四边形：左右两端斜 sk 像素
function slab(g, x0, x1, y, h, sk, c) { g.poly([[x0 + sk, y], [x1, y], [x1 - sk, y + h], [x0, y + h]], c); }
function slabR(g, x0, x1, y, h, sk, c) { g.poly([[x0, y], [x1 - sk, y], [x1, y + h], [x0 + sk, y + h]], c); }

function bar(g, f, side, t) {
  var P = 50, mid = W / 2, gap = 38;
  var x0 = side ? mid + gap : L + P, x1 = side ? Rr - P : mid - gap, y = TOP + 4, h = 17, sk = 9, len = x1 - x0;
  var sl = side ? slabR : slab;
  // 框：外面一圈亮边、里面暗底
  sl(g, x0 - 3, x1 + 3, y - 3, h + 6, sk, '#d8c18a');
  sl(g, x0 - 1.5, x1 + 1.5, y - 1.5, h + 3, sk, '#3a2a14');
  sl(g, x0, x1, y, h, sk, '#1a0f10');
  var k = Math.max(0, f.shown / D.HP), kt = Math.max(0, f.trail / D.HP);
  // 从外侧往中间：P1 左端固定，P2 右端固定（伤害吃在靠中间那头，与街机一致）
  function seg(frac, c, yy, hh) {
    if (frac <= 0.001) return;
    if (side) sl(g, x1 - len * frac, x1, yy, hh, sk * hh / h, c); else sl(g, x0, x0 + len * frac, yy, hh, sk * hh / h, c);
  }
  seg(kt, '#ffffff', y, h);
  var low = k < 0.3, blink = low && Math.floor(t / 8) % 2;
  seg(k, blink ? '#ff3a2a' : '#ff8a00', y, h);
  seg(k, blink ? '#ff6a50' : '#ffd23a', y, h * 0.55);
  seg(k, 'rgba(255,255,255,0.35)', y + 1, h * 0.18);
  // 头像框
  var px = side ? Rr - P + 6 : L, py = TOP - 2, ps = 44;
  g.rrect(px - 2, py - 2, ps + 4, ps + 4, 6, f.def.color);
  g.rrect(px, py, ps, ps, 5, '#120c18');
  var a = f.art;
  if (a && a.head) {
    var hd = a.head;
    g.image(a.tex, hd[0], hd[1], hd[2], hd[2], px + 2, py + 2, ps - 4, ps - 4, side === 1);
  }
  // 名字
  g.text(f.def.full, side ? x1 - 4 : x0 + 4, y + h + 11, 12, '#ffffff', side ? 'right' : 'left', 'bold', ['rgba(0,0,0,0.85)', 1.5]);
  // 能量槽（3 格）
  var mx0 = side ? x1 - 150 : x0, my = y + h + 22, mw = 150, mh = 7, st = f.stocks(), part = (f.meter % D.STOCK) / D.STOCK;
  g.rrect(mx0 - 1, my - 1, mw + 2, mh + 2, 3, 'rgba(0,0,0,0.6)');
  var mc = st >= 3 ? (Math.floor(t / 5) % 2 ? '#ff5ad0' : '#ffffff') : '#36a8ff';
  var fill = st >= 3 ? 1 : part;
  if (fill > 0) g.rect(side ? mx0 + mw * (1 - fill) : mx0, my, mw * fill, mh, mc);
  g.text(st >= 3 ? 'MAX' : '能量', side ? mx0 - 6 : mx0 + mw + 6, my + 3, 9, st >= 3 ? '#ff9ae6' : '#9fd6ff', side ? 'right' : 'left', 'bold', ['rgba(0,0,0,0.8)', 1]);
  for (var i = 0; i < 3; i++) {
    var cx = side ? mx0 - 44 + i * 11 : mx0 + mw + 34 + i * 11;
    g.circle(cx, my + 3, 4, i < st ? '#ff5ad0' : 'rgba(255,255,255,0.18)');
  }
}

function timer(g, fight, t) {
  var cx = W / 2, y = TOP - 4, sec = Math.ceil(fight.timer / 60);
  g.poly([[cx - 30, y], [cx + 30, y], [cx + 24, y + 44], [cx - 24, y + 44]], '#d8c18a');
  g.poly([[cx - 27, y + 2], [cx + 27, y + 2], [cx + 21.5, y + 42], [cx - 21.5, y + 42]], '#160c0c');
  var hot = sec <= 10 && fight.phase === 'fight' && Math.floor(t / 15) % 2;
  g.text(String(sec < 10 ? '0' + sec : sec), cx, y + 21, 26, hot ? '#ff4a3a' : '#ffe9a8', 'center', 'bold', ['#5a3010', 2]);
  // 回合胜标
  for (var s = 0; s < 2; s++) for (var i = 0; i < 2; i++) {
    var x = s ? cx + 40 + i * 14 : cx - 40 - i * 14, yy = TOP + 34;
    g.circle(x, yy, 5.5, '#2a1a10');
    g.circle(x, yy, 4, i < fight.wins[s] ? '#ffcc33' : 'rgba(255,255,255,0.15)');
  }
}

function toScreen(fight, x, y) {
  var s = H / Fight.VH * fight.cam.zoom, floorY = H - (900 - fight.stage.floor) * (H / Fight.VH);
  return [W / 2 + (x - fight.cam.x) * s, floorY - y * s - (fight.cam.zoom - 1) * 120];
}

function combo(g, fight) {
  for (var s = 0; s < 2; s++) {
    var c = fight.comboShow[s];
    if (c.t <= 0) continue;
    var a = Math.min(1, c.t / 15), slide = Math.max(0, (c.t - 55) / 15);
    var x = s ? Rr - 70 + slide * 80 : L + 70 - slide * 80, y = H * 0.36;
    g.save(); g.setAlpha(a);
    g.text(String(c.n), x, y, 44, '#ffe14a', 'center', 'bold', ['#8a1a00', 3]);
    g.text('连击', x, y + 32, 15, '#ffffff', 'center', 'bold', ['#8a1a00', 2]);
    g.text('HITS', x, y - 30, 12, '#ffb04a', 'center', 'bold', ['rgba(0,0,0,0.8)', 1.5]);
    g.restore();
  }
}

function callouts(g, fight) {
  fight.callouts.forEach(function (c) {
    if (c.big) return;
    var p = toScreen(fight, c.f.x, 290), a = c.t < 45 ? 1 : (60 - c.t) / 15, rise = Math.min(1, c.t / 8) * 12;
    g.save(); g.setAlpha(a);
    g.text(c.text, p[0], p[1] - rise, 18, '#ffffff', 'center', 'bold', [c.f.def.color, 2.5]);
    g.restore();
  });
}

function announce(g, fight) {
  var an = fight.announce;
  if (!an) return;
  var t = an.t, sc = t < 8 ? 2.2 - 1.2 * (t / 8) : 1, a = t < 4 ? t / 4 : 1;
  if (!an.ko && t > 50 && fight.phase !== 'intro') a = Math.max(0, 1 - (t - 50) / 30);
  if (a <= 0) return;
  var cx = W / 2, cy = H * 0.42;
  g.save(); g.setAlpha(a);
  // 背后一条横向光带
  g.gradRect(0, cy - 40, W, 80, [[0, 'rgba(0,0,0,0)'], [0.5, 'rgba(0,0,0,0.5)'], [1, 'rgba(0,0,0,0)']], false);
  g.setBlend('add');
  var lc = an.ko ? 'rgba(255,60,40,0.8)' : 'rgba(255,190,80,0.7)', lw = W * Math.min(1, t / 10);
  g.rect(cx - lw / 2, cy - 38, lw, 2, lc); g.rect(cx - lw / 2, cy + 36, lw, 2, lc);
  g.setBlend();
  g.translate(cx, cy); g.scale(sc, sc);
  if (an.ko) {
    var j = t < 20 ? (Math.random() - 0.5) * 6 : 0;
    g.setBlend('add'); g.glow(0, 0, 150, [1, 0.25, 0.1, 0.55]); g.setBlend();
    g.text('K.O.', j, 0, 66, '#ff3a1a', 'center', 'bold', ['#2a0400', 5]);
  } else {
    if (an.sub) g.text(an.sub, 0, -30, 16, '#ffd9a0', 'center', 'bold', ['rgba(0,0,0,0.8)', 2]);
    g.text(an.text, 0, 6, 46, an.hot ? '#ffcf3a' : '#ffffff', 'center', 'bold', [an.hot ? '#b01800' : '#2a3a8a', 4]);
  }
  g.restore();
}

// 超必杀：画面中间一条斜带切入角色大图 + 招式名
function superCut(g, fight, t) {
  if (!(fight.freeze > 0 && fight.superBy)) return;
  var f = fight.superBy, side = fight.p.indexOf(f), k = 46 - fight.freeze, a = Math.min(1, k / 6) * Math.min(1, fight.freeze / 6);
  var y = H * 0.3, h = H * 0.3, c = f.def.glow;
  g.save(); g.setAlpha(a);
  g.poly([[0, y + 16], [W, y - 16], [W, y + h - 16], [0, y + h + 16]], 'rgba(8,4,16,0.88)');
  g.setBlend('add');
  for (var i = 0; i < 14; i++) {
    var ly = y + ((i * 37 + k * 23) % h), lx = ((i * 131 + k * (side ? -40 : 40)) % (W + 200) + W + 200) % (W + 200) - 100;
    g.rect(lx, ly, 120 + (i % 3) * 60, 2, [c[0], c[1], c[2], 0.5]);
  }
  g.rect(0, y - 16 + 0, W, 3, [c[0], c[1], c[2], 0.9]);
  g.setBlend();
  var art = f.art;
  if (art && art.poses.hp) {
    var p = art.poses.hp, s = h * 1.25 / p[3], dir = side ? -1 : 1;
    var slide = Math.max(0, 1 - k / 8) * W * 0.3 * -dir;
    var px = (side ? W * 0.62 : W * 0.38) - p[2] * s / 2 + slide;
    g.image(art.tex, p[0], p[1], p[2], p[3], px, y + h - p[3] * s + 6, p[2] * s, p[3] * s, side === 1);
  }
  var m = f.moves.su;
  g.text(m.name, side ? W * 0.28 : W * 0.72, y + h * 0.5, 34, '#ffffff', 'center', 'bold', [f.def.color, 4]);
  g.text('超必杀技', side ? W * 0.28 : W * 0.72, y + h * 0.5 - 32, 13, '#ffd9f0', 'center', 'bold', ['rgba(0,0,0,0.8)', 2]);
  g.restore();
}

function draw(g, fight, t) {
  bar(g, fight.p[0], 0, t);
  bar(g, fight.p[1], 1, t);
  timer(g, fight, t);
  combo(g, fight);
  callouts(g, fight);
  superCut(g, fight, t);
  announce(g, fight);
}

module.exports = { draw: draw, TOP: TOP };
