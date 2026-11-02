// 泡泡龙（竖屏）：按住屏幕瞄准（瞄准线会算好墙面反弹、画出落点），松手发射；三个及以上同色连在一起就消除，
// 因此悬空的泡泡整串掉落（掉落分翻倍）。连续几发没消除，顶上就压下来一行；泡泡越过底部虚线就输，可以看广告复活一次。
// 打光全部泡泡过关，关卡进度与最高分存档；点左下角「下一个」换球。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var Grid = require('js/grid.js');
var Shot = require('js/shot.js');
var Levels = require('js/levels.js');
var Planner = require('js/ai.js');
var Social = require('js/social.js');
var View = require('js/view.js');
var W = B.W, H = B.H;
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);
View.setup(B);

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';
var COLS = 10, LEFT = 8, RAD = (W - 16) / (COLS * 2);
var FT = B.hudTop + 50, SX = W / 2, SY = H - 86, DY = SY - RAD * 2.6;
var grid = new Grid(COLS, LEFT, FT, RAD);

var S = {
  scene: 'menu', level: 1, L: null, score: 0, levelScore: 0, best: Number(B.load('bubble.best', 0)) || 0,
  unlocked: Math.max(1, Number(B.load('bubble.level', 1)) || 1), shots: 0, pushLeft: 0, revived: false,
  cur: 0, nxt: 1, aim: { on: false, ang: Math.PI / 2, sim: null, ver: -1 }, fly: null, pushAt: -9, clock: 0, sceneAt: 0,
  pops: [], falls: [], msg: null, msgAt: -9, stars: 0, startCount: 0, paused: false, buttons: {},
  invite: q.from === 'share' ? Math.max(1, Number(q.level) || 1) : 0, popped: 0, dropped: 0, pushes: 0, clears: 0, overs: 0,
  geo: { SX: SX, SY: SY, DY: DY, FT: FT, RAD: RAD, COLS: COLS, LEFT: LEFT }
};
var social = Social.make(function () {
  return { title: S.scene === 'clear' ? '我在泡泡龙通关了第 ' + S.level + ' 关「' + S.L.name + '」，拿了 ' + S.score + ' 分，来比比？'
    : '泡泡龙第 ' + S.level + ' 关好难，你能打到几分？', query: 'from=share&level=' + S.level };
});
S.reviveLabel = social.reviveLabel;

// ───────── 关卡 ─────────
function startLevel(n, keepScore) {
  S.level = n; S.L = Levels.build(grid, n);
  if (!keepScore) S.score = 0;
  S.levelScore = S.score; S.shots = 0; S.pushLeft = S.L.push; S.revived = false; S.fly = null;
  S.pops.length = 0; S.falls.length = 0; S.startCount = grid.count();
  S.cur = pickColor(); S.nxt = pickColor();
  S.aim.on = false; S.aim.ver = -1;
  setScene('play');
  say('第 ' + n + ' 关 · ' + S.L.name);
  planner.reset(); autoAt = S.clock + 0.6;
}
function setScene(s) { S.scene = s; S.sceneAt = S.clock; }
function pickColor() {
  var cs = grid.colors();
  if (!cs.length) return Math.floor(Math.random() * S.L.colors);
  return cs[Math.floor(Math.random() * cs.length)];
}
function say(t) { S.msg = t; S.msgAt = S.clock; }

// ───────── 瞄准与发射 ─────────
function setAim(x, y) {
  var a = Math.atan2(SY - y, x - SX);
  if (a < 0.14) a = 0.14; if (a > Math.PI - 0.14) a = Math.PI - 0.14;
  S.aim.ang = a; S.aim.ver = -1;
}
function aimSim() {
  if (S.aim.ver !== grid.version || S.aim.simAng !== S.aim.ang) {
    S.aim.sim = Shot.simulate(grid, SX, SY, S.aim.ang); S.aim.ver = grid.version; S.aim.simAng = S.aim.ang;
  }
  return S.aim.sim;
}
function shoot() {
  if (S.scene !== 'play' || S.fly || S.paused) return;
  var sim = aimSim();
  if (!sim.cell) return;
  S.fly = { pts: sim.pts, cell: sim.cell, color: S.cur, s: 0, pos: [SX, SY, 0] };
  S.cur = S.nxt; S.nxt = pickColor(); S.shots++;
  B.beep(520, 0.07, 'sine', 0.12, 1.4);
}
function swap() { if (S.fly) return; var t = S.cur; S.cur = S.nxt; S.nxt = t; B.beep(700, 0.04, 'triangle', 0.08); }

function land(f) {
  var i = f.cell[0], c = f.cell[1];
  grid.set(i, c, f.color);
  var cl = grid.cluster(i, c), gain = 0;
  if (cl.length >= 3) {
    cl.forEach(function (p) { pop(grid.x(p[0], p[1]), grid.y(p[0]), grid.get(p[0], p[1])); grid.set(p[0], p[1], -1); });
    gain += cl.length * 10; S.popped += cl.length;
    var fl = grid.floating();
    fl.forEach(function (p) { fall(grid.x(p[0], p[1]), grid.y(p[0]), grid.get(p[0], p[1])); grid.set(p[0], p[1], -1); });
    if (fl.length) { gain += fl.length * 20 * (fl.length >= 6 ? 2 : 1); S.dropped += fl.length; }
    S.score += gain;
    say(fl.length ? '+' + gain + '  掉落 ' + fl.length + ' 个' + (fl.length >= 6 ? ' ×2' : '') : '+' + gain);
    B.beep(660 + Math.min(cl.length, 8) * 60, 0.12, 'triangle', 0.13, 1.5);
    if (fl.length) setTimeout(function () { B.beep(330, 0.25, 'sine', 0.12, 0.5); }, 120);
  } else {
    B.beep(240, 0.06, 'square', 0.06);
    if (--S.pushLeft <= 0) pushDown();
  }
  if (!grid.count()) return levelClear();
  if (lost()) return gameOver();
  // 手里的颜色已经不在场上了：换成场上有的（否则这一发怎么打都消不掉）
  var cs = grid.colors();
  if (cs.indexOf(S.cur) < 0) S.cur = pickColor();
  if (cs.indexOf(S.nxt) < 0) S.nxt = pickColor();
}
function pushDown() {
  var cs = grid.colors(); if (!cs.length) cs = [0, 1, 2];
  grid.pushRow(function () { return cs[Math.floor(Math.random() * cs.length)]; });
  S.pushLeft = S.L.push; S.pushAt = S.clock; S.pushes++;
  B.beep(150, 0.2, 'sawtooth', 0.1, 0.7); B.buzz();
  say('压下来一行！');
}
function lost() { var low = grid.lowest(); return low >= 0 && grid.y(low) + RAD > DY; }
function levelClear() {
  var par = Math.ceil(S.startCount / 2.6);
  S.stars = S.shots <= par ? 3 : S.shots <= par * 1.6 ? 2 : 1;
  var bonus = 300 + S.stars * 200;
  S.score += bonus; S.bonus = bonus; S.clears++;
  if (S.score > S.best) { S.best = S.score; B.save('bubble.best', S.best); }
  if (S.level + 1 > S.unlocked) { S.unlocked = S.level + 1; B.save('bubble.level', S.unlocked); }
  setScene('clear');
  [523, 659, 784, 1047].forEach(function (f, k) { setTimeout(function () { B.beep(f, 0.12, 'triangle', 0.12); }, k * 90); });
  console.log('[bubble] 第 ' + S.level + ' 关通关：' + S.shots + ' 发，' + S.stars + ' 星，总分 ' + S.score);
}
function gameOver() {
  S.overs++; S.newBest = S.score > S.best;
  if (S.newBest) { S.best = S.score; B.save('bubble.best', S.best); }
  setScene('over');
  B.beep(300, 0.5, 'sawtooth', 0.12, 0.35); B.buzz();
  console.log('[bubble] 第 ' + S.level + ' 关失败：' + S.shots + ' 发，总分 ' + S.score + '，复活过 ' + (S.revived ? '是' : '否') + '，广告 ' + social.adState());
}
function revive() {
  if (S.revived || S.reviving) return;
  S.reviving = true;                                   // 广告 show / load 是异步的：等结果期间别重复点
  social.revive(function (how) {
    S.reviving = false;
    if (!how || S.scene !== 'over') return;
    S.revived = true; grid.dropBottom(3); S.pushLeft = S.L.push;
    setScene('play'); say(how === 'ad' ? '复活成功！' : '免费复活一次！');
    planner.reset(); autoAt = S.clock + 0.6;
  });
}

// ───────── 动画粒子（对象池：数组只增不减，用 t < 0 表示空闲） ─────────
function slot(list) { for (var i = 0; i < list.length; i++) if (list[i].t < 0) return list[i]; var o = {}; list.push(o); return o; }
function pop(x, y, k) { var p = slot(S.pops); p.x = x; p.y = y; p.k = k; p.t = 0; }
function fall(x, y, k) { var p = slot(S.falls); p.x = x; p.y = y; p.k = k; p.t = 0; p.vx = (Math.random() - 0.5) * 120; p.vy = -60 - Math.random() * 80; }

// ───────── 每帧 ─────────
var planner = new Planner(), autoAt = 0, autoAimAt = -1, autoSwapped = false;
function update(dt) {
  S.clock += dt;
  if (S.paused) return;
  var i, p;
  for (i = 0; i < S.pops.length; i++) { p = S.pops[i]; if (p.t >= 0) { p.t += dt; if (p.t > 0.22) p.t = -1; } }
  for (i = 0; i < S.falls.length; i++) {
    p = S.falls[i]; if (p.t < 0) continue;
    p.t += dt; p.vy += 1500 * dt; p.x += p.vx * dt; p.y += p.vy * dt;
    if (p.y > H + RAD) p.t = -1;
  }
  if (S.scene === 'play' && S.fly) {
    S.fly.s += 1150 * dt;
    Shot.along(S.fly.pts, S.fly.s, S.fly.pos);
    if (S.fly.pos[2]) { var f = S.fly; S.fly = null; land(f); }
  }
  if (AUTO) autoStep();
}
function autoStep() {
  // ?auto=1&level=N：自动试玩直接从第 N 关打（测高难度关卡 / 复活与结算那条路）
  if (S.scene === 'menu' && S.clock > 0.4) return startLevel(Number(q.level) > 0 && q.from !== 'share' ? Number(q.level) : S.unlocked);
  if (S.scene === 'clear' && !S.autoShared && S.clock - S.sceneAt > 0.8) { S.autoShared = true; social.share(); }   // 走一遍主动转发
  if (S.scene === 'clear' && S.clock - S.sceneAt > 1.6) return startLevel(S.level + 1, true);
  if (S.scene === 'over' && S.clock - S.sceneAt > 1.4) { if (!S.revived) revive(); else startLevel(S.level); return; }
  if (S.scene !== 'play' || S.fly || S.clock < autoAt) return;
  if (autoAimAt < 0) {
    if (!planner.step(grid, SX, SY, S.cur, 16)) return;
    var best = planner.pick();
    if (best.score < 60 && !autoSwapped && S.nxt !== S.cur) { autoSwapped = true; swap(); planner.reset(); return; }
    S.aim.ang = best.ang + (Math.random() - 0.5) * 0.01; S.aim.on = true; autoAimAt = S.clock;
    return;
  }
  if (S.clock - autoAimAt > 0.3) {
    shoot(); S.aim.on = false; autoAimAt = -1; autoSwapped = false; planner.reset(); autoAt = S.clock + 0.25;
  }
}

// ───────── 输入 ─────────
function nextBox() { return { x: SX - 74 - RAD - 6, y: SY + 22 - RAD - 6, w: RAD * 2 + 12, h: RAD * 2 + 22 }; }
B.on('down', function (x, y) {
  if (S.paused) { S.paused = false; return; }
  if (S.scene !== 'play' || AUTO) return;
  if (B.hit(nextBox(), x, y)) { swap(); return; }
  if (y < SY - 12) { S.aim.on = true; setAim(x, y); }
});
B.on('move', function (x, y) { if (S.aim.on && !AUTO) setAim(x, y); });
B.on('up', function (x, y) {
  if (!S.aim.on || AUTO) return;
  S.aim.on = false;
  if (y < SY - 12) shoot();                            // 拖回炮台下方再松手 = 取消这一发
});
B.on('tap', function (x, y) {
  var b = S.buttons;
  if (S.scene === 'menu') {
    if (B.hit(b.cont, x, y)) startLevel(S.unlocked);
    else if (B.hit(b.fresh, x, y)) startLevel(1);
    else if (B.hit(b.invite, x, y)) startLevel(Math.min(S.invite, S.unlocked));
    else if (B.hit(b.share, x, y)) social.share();
  } else if (S.scene === 'over') {
    if (B.hit(b.revive, x, y)) revive();
    else if (B.hit(b.retry, x, y)) { S.score = S.levelScore; startLevel(S.level, true); }
    else if (B.hit(b.share, x, y)) social.share();
    else if (B.hit(b.home, x, y)) setScene('menu');
  } else if (S.scene === 'clear') {
    if (B.hit(b.next, x, y)) startLevel(S.level + 1, true);
    else if (B.hit(b.share, x, y)) social.share();
  }
});
B.on('key', function (k) {
  if (k === 'pause') { if (S.scene === 'play') S.paused = !S.paused; return; }
  if (S.paused) { S.paused = false; return; }
  if (S.scene === 'play') {
    if (k === 'left' || k === 'right') { S.aim.on = true; S.aim.ang = Math.max(0.14, Math.min(Math.PI - 0.14, S.aim.ang + (k === 'left' ? 0.035 : -0.035))); }
    else if (k === 'ok') { shoot(); S.aim.on = false; }
    else if (k === 'up' || k === 'down') swap();
  } else if (k === 'ok') {
    if (S.scene === 'menu') startLevel(S.unlocked);
    else if (S.scene === 'clear') startLevel(S.level + 1, true);
    else if (S.scene === 'over') { S.score = S.levelScore; startLevel(S.level, true); }
  }
});
B.onHide(function () { if (S.scene === 'play') S.paused = true; S.aim.on = false; });

GameGlobal.__play = function () {
  return {
    scene: S.scene, level: S.level, levelName: S.L ? S.L.name : null, score: S.score, best: S.best, unlocked: S.unlocked,
    shots: S.shots, bubbles: grid.count(), rows: grid.lowest() + 1, pushLeft: S.pushLeft, popped: S.popped, dropped: S.dropped,
    pushes: S.pushes, clears: S.clears, overs: S.overs, revived: S.revived, ad: social.adState(), social: social.stats
  };
};

B.loop(update, function () { View.draw(R, S, grid, aimSim, nextBox()); });
