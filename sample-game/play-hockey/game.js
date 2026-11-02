// 桌上冰球（横屏）：双人同屏（左右各一根手指同时拖自己的球拍）或人机对战（三档难度），先得 7 分获胜。
// 多点触控按 identifier 跟踪：每根手指按下时认领它那一侧的球拍，移动 / 抬起只认这根手指，两根手指互不干扰。
// 物理固定步长 240Hz（js/physics.js），电脑在 js/ai.js，画面 js/draw.js（WebGL 批量三角形，js/gl2d.js）。
// 碰撞音走 WebAudio 预合成 AudioBuffer（js/base.js 的 beep），按撞击力度分四档音高。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var P = require('js/physics.js');
var AI = require('js/ai.js');
var D = require('js/draw.js');
var W = B.W, H = B.H;
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);
var WIN = 7;

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

// 冰面放在安全区里：横屏刘海在左或右，顶部留给比分条与右上角胶囊
var safeL = Math.max(B.safe.left || 0, 0), safeR = Math.min(B.safe.right || W, W);
var barY = Math.max(B.safe.top || 0, 6) + 18, barRight = safeR - 12;
try {
  var menu = wx.getMenuButtonBoundingClientRect && wx.getMenuButtonBoundingClientRect();
  if (menu && menu.left > W / 2 && menu.bottom < H / 3) { barRight = Math.min(barRight, menu.left - 10); barY = Math.max(barY, menu.top + menu.height / 2); }
} catch (e) {}
var rinkTop = barY + 30, rinkBottom = Math.min(B.safe.bottom || H, H) - 16;
var rink = P.makeRink(safeL + 28, rinkTop, safeR - 28, rinkBottom);
var world = P.makeWorld(rink, onPhys);
var view = D.make(R, rink, H);
var PAUSE_BTN = { x: W / 2 - 18, y: barY - 15, w: 36, h: 30 };

var state = 'menu', mode = 'ai', level = 1, scores = [0, 0], clock = 0, overAt = 0, userPaused = false, winner = 0;
var bots = [null, null], banner = '', bannerT = 0, hits = 0, walls = 0, goals = 0, matches = 0, lastSound = 0, newRecord = false;
var stats = B.load('hockey.stats', null) || { win: 0, lose: 0, streak: 0, bestStreak: 0 };
var buttons = {}, lastRender = 0;

function startMatch(m, lv) {
  mode = m; level = lv === undefined ? level : lv;
  scores = [0, 0]; winner = 0; userPaused = false; newRecord = false;
  world.resetMallets(); world.mallets.forEach(function (x) { x.touch = null; x.maxSpeed = 0; });
  // 人机：右边是电脑；自动试玩：左边也交给电脑（两边对打）
  bots[1] = mode === 'ai' ? AI.make(world, world.mallets[1], level) : null;
  bots[0] = AUTO ? AI.make(world, world.mallets[0], Math.max(0, 2 - level)) : null;
  world.serve(Math.random() < 0.5 ? -1 : 1, 0.6);
  view.clearTrail();
  state = 'play';
  say(mode === 'ai' ? '对战电脑 · ' + AI.LEVELS[level].name : '双人对战', 1.2);
}
function say(s, t) { banner = s; bannerT = t || 1.2; }

// ───────── 物理事件：音效、光圈、进球 ─────────
function onPhys(kind, a, x, y, side) {
  if (kind === 'goal') return goal(a);
  if (clock - lastSound < 0.05) return;                  // 同一瞬间连撞几次只响一声
  if (kind === 'mallet') {
    hits++; lastSound = clock;
    var lv = a > 900 ? 3 : a > 500 ? 2 : a > 200 ? 1 : 0;
    B.beep([520, 640, 780, 960][lv], 0.05, 'triangle', 0.06 + lv * 0.03, 0.8);
    if (lv >= 2) view.ring(x, y, side < 0 ? D.COLORS.redL : D.COLORS.blueL);
  } else if (a > 120) {
    walls++; lastSound = clock;
    B.beep(kind === 'post' ? 1500 : 330, 0.04, kind === 'post' ? 'square' : 'sine', Math.min(0.12, 0.03 + a / 12000));
  }
}
function goal(scorer) {
  var i = scorer < 0 ? 0 : 1;
  scores[i]++; goals++;
  B.buzz();
  [660, 880, 1320].forEach(function (f, k) { setTimeout(function () { B.beep(f, 0.09, 'square', 0.08); }, k * 80); });
  view.ring(scorer < 0 ? rink.R : rink.L, rink.cy, '#ffd54f');
  if (scores[i] >= WIN) return finish(scorer);
  say((i === 0 ? (mode === 'ai' && !AUTO ? '你' : '红方') : (mode === 'ai' ? '电脑' : '蓝方')) + '得分！ ' + scores[0] + ' : ' + scores[1], 1);
  world.serve(-scorer, 1.0);                              // 丢球的一方开球
  view.clearTrail();
}
function finish(scorer) {
  state = 'over'; overAt = clock; winner = scorer; matches++;
  world.serve(0, 999);
  if (mode === 'ai') {
    if (scorer < 0) { stats.win++; stats.streak++; if (stats.streak > stats.bestStreak) { stats.bestStreak = stats.streak; newRecord = true; } }
    else { stats.lose++; stats.streak = 0; }
    B.save('hockey.stats', stats);
  }
  [523, 659, 784, 1047].forEach(function (f, k) { setTimeout(function () { B.beep(f, 0.14, 'triangle', 0.1); }, k * 120); });
}

// ───────── 输入：按 identifier 跟踪每根手指 ─────────
function malletFor(id) {
  var ms = world.mallets;
  for (var i = 0; i < 2; i++) if (ms[i].touch === id) return ms[i];
  return null;
}
B.on('down', function (x, y, id) {
  if (state !== 'play') return;
  if (userPaused) return;
  if (B.hit(PAUSE_BTN, x, y)) { userPaused = true; return; }
  // 双人：按在哪半场就拿哪只拍；人机：玩家只有左边那只，按哪都拿它
  var m = mode === 'pvp' ? world.mallets[x < rink.mid ? 0 : 1] : world.mallets[0];
  if (bots[m.side < 0 ? 0 : 1]) return;
  if (m.touch !== null) return;                           // 这只拍已经有手指了：第二根手指不抢
  m.touch = id; m.tx = x; m.ty = y;
});
B.on('move', function (x, y, id) {
  var m = malletFor(id);
  if (m) { m.tx = x; m.ty = y; }
});
B.on('up', function (x, y, id) {
  var m = malletFor(id);
  if (m) m.touch = null;
});
B.on('tap', function (x, y) {
  if (state === 'menu') {
    if (B.hit(buttons.ai, x, y)) startMatch('ai');
    else if (B.hit(buttons.pvp, x, y)) startMatch('pvp');
    for (var i = 0; i < 3; i++) if (B.hit(buttons['lv' + i], x, y)) { level = i; B.beep(880, 0.04, 'triangle', 0.06); }
  } else if (state === 'over') {
    if (clock - overAt < 0.6) return;
    if (B.hit(buttons.again, x, y)) startMatch(mode);
    else if (B.hit(buttons.back, x, y)) state = 'menu';
  } else if (userPaused) {
    if (B.hit(buttons.resume, x, y)) userPaused = false;
    else if (B.hit(buttons.restart, x, y)) startMatch(mode);
    else if (B.hit(buttons.quit, x, y)) { userPaused = false; state = 'menu'; }
  }
});
B.on('key', function (k) {
  if (k === 'pause') { if (state === 'play') userPaused = !userPaused; return; }
  if (k === 'ok') {
    if (state === 'menu') startMatch('ai');
    else if (state === 'over' && clock - overAt > 0.6) startMatch(mode);
    else userPaused = false;
  }
});
B.onHide(function () {
  if (state === 'play') userPaused = true;
  world.mallets.forEach(function (m) { m.touch = null; });  // 切后台时手指的抬起事件不会来
});

// ───────── 更新 ─────────
function update(dt) {
  clock += dt;
  if (bannerT > 0) bannerT -= dt;
  if (AUTO && state === 'menu') startMatch('ai', 1);
  if (AUTO && state === 'over' && clock - overAt > 2) startMatch('ai', (level + 1) % 3);
  if (state !== 'play' || userPaused) return;
  // PC：方向键 / WASD 推左边的拍
  var km = world.mallets[0], kx = (B.held.right ? 1 : 0) - (B.held.left ? 1 : 0), ky = (B.held.down ? 1 : 0) - (B.held.up ? 1 : 0);
  if ((kx || ky) && !bots[0] && km.touch === null) { km.tx = km.x + kx * 40; km.ty = km.y + ky * 40; }
  if (bots[0]) bots[0].think(dt);
  if (bots[1]) bots[1].think(dt);
  world.advance(dt);
  // 球停在没人够得着的地方太久（自动试玩两边都在等）：重新开球
  if (world.idle() > 4) { world.serve(Math.random() < 0.5 ? -1 : 1, 0.5); say('重新开球', 0.8); }
}

// ───────── 绘制 ─────────
function render() {
  R.begin(view.ice);                                     // 清屏色 = 冰面（见 draw.js 的 clears）
  var dt = Math.max(0, clock - lastRender); lastRender = clock;
  view.frame(world, scores, clock, dt);
  drawBar();
  if (bannerT > 0 && state === 'play') {
    R.setAlpha(Math.min(1, bannerT * 2));
    R.rrect(W / 2 - 130, rink.T + 16, 260, 40, 20, 'rgba(0,0,0,0.55)');
    R.text(banner, W / 2, rink.T + 36, 18, '#ffffff', 'center', 'bold');
    R.setAlpha(1);
  }
  if (state === 'menu') drawMenu();
  else if (state === 'over') drawOver();
  else if (userPaused) drawPause();
  R.end();
}
function drawBar() {
  var left = mode === 'ai' && !AUTO ? '你' : '红方', right = mode === 'ai' ? '电脑 · ' + AI.LEVELS[level].name : '蓝方';
  R.circle(rink.L + 8, barY, 7, D.COLORS.red);
  R.text(left, rink.L + 22, barY, 15, '#ffffff', 'left', 'bold');
  R.text(String(scores[0]), W / 2 - 36, barY, 22, '#ff8a80', 'right', 'bold');
  R.text(String(scores[1]), W / 2 + 36, barY, 22, '#82b1ff', 'left', 'bold');
  R.text(right, barRight - 18, barY, 15, '#ffffff', 'right', 'bold');
  R.circle(barRight - 6, barY, 7, D.COLORS.blue);
  if (state === 'play') {
    R.rrect(PAUSE_BTN.x, PAUSE_BTN.y, PAUSE_BTN.w, PAUSE_BTN.h, 15, 'rgba(255,255,255,0.15)');
    R.rect(W / 2 - 6, barY - 7, 3, 14, '#ffffff'); R.rect(W / 2 + 3, barY - 7, 3, 14, '#ffffff');
  } else R.text(':', W / 2, barY, 20, '#ffffff', 'center', 'bold');
}
function panel(w, h) { R.rrect(W / 2 - w / 2, H / 2 - h / 2 + 10, w, h, 18, 'rgba(16,32,39,0.88)'); }
function drawMenu() {
  var cx = W / 2, cy = H / 2 + 10;
  panel(380, 250);
  R.text('桌上冰球', cx, cy - 92, 30, '#ffffff', 'center', 'bold');
  R.text('拖动球拍击球，先得 ' + WIN + ' 分获胜', cx, cy - 58, 14, '#b0bec5', 'center');
  buttons.ai = R.button('对战电脑', cx - 88, cy - 12, 150, 44, D.COLORS.blue, '#ffffff');
  buttons.pvp = R.button('双人同屏', cx + 88, cy - 12, 150, 44, D.COLORS.red, '#ffffff');
  for (var i = 0; i < 3; i++) {
    var on = i === level, bx = cx - 100 + i * 100;
    buttons['lv' + i] = R.button(AI.LEVELS[i].name, bx, cy + 44, 84, 32, on ? '#ffd54f' : 'rgba(255,255,255,0.14)', on ? '#3e2723' : '#eceff1');
  }
  R.text('人机战绩  胜 ' + stats.win + '  负 ' + stats.lose + '   最高连胜 ' + stats.bestStreak, cx, cy + 90, 14, '#ffe082', 'center');
}
function drawOver() {
  var cx = W / 2, cy = H / 2 + 10, me = mode === 'ai' && !AUTO;
  panel(360, 220);
  var title = me ? (winner < 0 ? '你赢了！' : '电脑赢了') : (winner < 0 ? '红方获胜！' : '蓝方获胜！');
  R.text(title, cx, cy - 72, 30, winner < 0 ? '#ff8a80' : '#82b1ff', 'center', 'bold');
  R.text(scores[0] + ' : ' + scores[1], cx, cy - 30, 26, '#ffffff', 'center', 'bold');
  if (mode === 'ai') R.text((newRecord ? '新纪录！' : '') + '连胜 ' + stats.streak + ' · 最高连胜 ' + stats.bestStreak + ' · 胜 ' + stats.win + ' 负 ' + stats.lose, cx, cy + 4, 14, newRecord ? '#ffd54f' : '#cfd8dc', 'center');
  var show = clock - overAt > 0.6 ? 1 : 0.4;
  R.setAlpha(show);
  buttons.again = R.button('再来一局', cx - 80, cy + 56, 140, 42, '#ffd54f', '#3e2723');
  buttons.back = R.button('返回菜单', cx + 80, cy + 56, 140, 42, 'rgba(255,255,255,0.16)', '#ffffff');
  R.setAlpha(1);
}
function drawPause() {
  var cx = W / 2, cy = H / 2 + 10;
  panel(300, 200);
  R.text('已暂停', cx, cy - 62, 26, '#ffffff', 'center', 'bold');
  buttons.resume = R.button('继续', cx, cy - 14, 180, 40, '#ffd54f', '#3e2723');
  buttons.restart = R.button('重新开始', cx - 66, cy + 44, 120, 36, 'rgba(255,255,255,0.16)', '#ffffff');
  buttons.quit = R.button('返回菜单', cx + 66, cy + 44, 120, 36, 'rgba(255,255,255,0.16)', '#ffffff');
}

GameGlobal.__play = function () {
  var p = world.puck, ms = world.mallets;
  return {
    state: userPaused ? 'paused' : state, mode: mode, level: AI.LEVELS[level].name, score: scores.slice(), winner: winner,
    matches: matches, goals: goals, hits: hits, walls: walls, stats: stats,
    puck: { x: Math.round(p.x), y: Math.round(p.y), v: Math.round(Math.sqrt(p.vx * p.vx + p.vy * p.vy)) },
    mallets: ms.map(function (m) { return { x: Math.round(m.x), y: Math.round(m.y), touch: m.touch }; }),
    rink: [rink.L, rink.T, rink.R, rink.B], w: W, h: H
  };
};
// 无头调试：按固定步长推进 sec 秒；__start('pvp' | 'ai', 难度) 直接开一局
GameGlobal.__sim = function (sec) { for (var i = 0; i < Math.round(sec * 60); i++) update(1 / 60); return GameGlobal.__play(); };
GameGlobal.__start = function (m, lv) { startMatch(m, lv); return GameGlobal.__play(); };

B.loop(update, render);
