// 切水果（竖屏）：水果从底下抛上来，手指划过就切成两半（果汁四溅、留下果渍），0.28 秒内接连切到 3 个以上算连切、奖励个数分。
// 经典模式：漏掉 3 个水果或切到炸弹结束；限时模式：60 秒，炸弹扣 10 分。菜单上划开西瓜 / 橙子选模式。
// 刀光：wx.onTouchMove 的每个点进环形缓冲，每一段「上一点 → 这一点」与每个水果做线段—圆相交（js/blade.js、js/world.js）。
// 切开 wx.vibrateShort（限频）、切到炸弹 wx.vibrateLong；音效 WebAudio 预合成（js/base.js）。切后台暂停，回前台倒数 3 秒继续。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var Wd = require('js/world.js');
var Blade = require('js/blade.js');
var AI = require('js/ai.js');
var Rn = require('js/render.js');
var W = B.W, H = B.H, S = Wd.S, UI = Rn.UI;
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);
var view = Rn.make(R);

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';
S.best.classic = Number(B.load('fruit.best.classic', 0)) || 0;
S.best.timed = Number(B.load('fruit.best.timed', 0)) || 0;
S.aiNext = q.mode === 'timed' ? 'timed' : 'classic';            // ?auto=1&mode=timed 先试玩限时模式
var P = { paused: false, countdown: 0, hidden: false, muted: B.load('fruit.muted', false) === true };
B.setMuted(P.muted);
var lastDraws = 0, lastBuzz = -1, lastSwish = -1;

// ───────── 声音与震动 ─────────
function later(f, ms) { setTimeout(f, ms); }
Wd.setFx(function (name, n) {
  if (name === 'cut') {
    B.beep(0, 0.12, 'noise', 0.07); B.beep(180 + Math.min(n, 8) * 30, 0.1, 'sine', 0.12, 0.6);   // 噗：果肉的闷响，连切音高往上走
    if (S.clock - lastBuzz > 0.08) { lastBuzz = S.clock; B.buzz(); }
  }
  else if (name === 'throw') B.beep(300, 0.12, 'sine', 0.04, 1.8);
  else if (name === 'combo') for (var i = 0; i < Math.min(n, 6); i++) (function (k) { later(function () { B.beep(660 * Math.pow(1.122, k), 0.09, 'triangle', 0.1); }, k * 60); })(i);
  else if (name === 'bomb') { B.beep(0, 1.0, 'noise', 0.22); B.beep(70, 0.8, 'sawtooth', 0.12, 0.4); }
  else if (name === 'bombSmall') { B.beep(0, 0.5, 'noise', 0.16); B.beep(90, 0.4, 'sawtooth', 0.1, 0.4); }
  else if (name === 'miss') B.beep(160, 0.3, 'square', 0.08, 0.6);
  else if (name === 'tick') B.beep(1000, 0.05, 'square', 0.05);
  else if (name === 'start') [523, 659, 784].forEach(function (f, i) { later(function () { B.beep(f, 0.08, 'triangle', 0.1); }, i * 70); });
  else if (name === 'over') B.beep(330, 0.5, 'triangle', 0.12, 0.5);
});

// ───────── 刀：真手指与自动试玩共用 ─────────
var hand = {
  down: function (x, y, id) { Blade.down(x, y, id, S.clock); },
  move: function (x, y, id) {
    var s = Blade.move(x, y, id, S.clock);
    if (!s) return;
    if (Blade.speed() > 18 && S.clock - lastSwish > 0.18) { lastSwish = S.clock; B.beep(0, 0.08, 'noise', 0.03); }
    Wd.slice(s.x0, s.y0, s.x1, s.y1);
  },
  up: function (id) { Blade.up(id); }
};
function canCut() { return !P.paused && (S.state === 'play' || S.state === 'ready'); }
B.on('down', function (x, y, id) {
  if (S.state === 'play' && !P.paused && B.hit(UI.pause, x, y)) { pause(); return; }
  if (canCut()) hand.down(x, y, id);
});
B.on('move', function (x, y, id) { if (canCut()) hand.move(x, y, id); });
B.on('up', function (x, y, id) { hand.up(id); });
B.on('tap', function (x, y) {
  if (S.state === 'ready') {                          // 点一下菜单水果也行（不会划的玩家）
    for (var i = 0; i < S.menu.length; i++) {
      var m = S.menu[i], dx = x - m.x, dy = y - m.y;
      if (m.on && !S.menuGo && dx * dx + dy * dy < 50 * 50) Wd.slice(m.x - 40, m.y + 10, m.x + 40, m.y - 10);
    }
  } else if (S.state === 'over') { if (B.hit(UI.a, x, y)) Wd.reset(S.mode); else if (B.hit(UI.b, x, y)) Wd.toMenu(); }
  else if (P.paused) {
    if (B.hit(UI.a, x, y)) resume();
    else if (B.hit(UI.b, x, y)) { P.muted = !P.muted; B.setMuted(P.muted); B.save('fruit.muted', P.muted); }
    else if (B.hit(UI.c, x, y)) { resume(); Wd.toMenu(); }
  }
});
B.on('key', function (k) {
  if (k === 'pause') { if (P.paused) resume(); else pause(); }
  else if (k === 'ok' && P.paused) resume();
  else if (k === 'ok' && S.state === 'over' && S.clock - S.overAt > 0.6) Wd.reset(S.mode);
});
function pause() { if (S.state === 'play' && !P.paused) { P.paused = true; P.countdown = 0; } }
function resume() { P.paused = false; P.countdown = 0; P.hidden = false; }
B.onHide(function () { if (S.state === 'play' && !P.paused) { P.paused = true; P.hidden = true; } });
B.onShow(function () { if (P.paused && P.hidden) { P.countdown = 3; P.hidden = false; } });

// ───────── 主循环 ─────────
function update(dt) {
  if (P.paused) {
    S.clock += dt;
    if (P.countdown > 0 && (P.countdown -= dt) <= 0) resume();
    return;
  }
  Wd.update(dt);
  if (AUTO) {
    AI.think(S, Wd, W, H, dt, hand);
    if (S.state === 'over' && S.clock - S.overAt > 1.5) { S.aiNext = S.mode === 'classic' ? 'timed' : 'classic'; Wd.toMenu(); }
  }
}
function render() { lastDraws = view.draw(P); }

GameGlobal.__play = function () {
  return {
    state: S.state, mode: S.mode, paused: P.paused, score: S.score, best: S.best, lives: S.lives,
    timeLeft: Math.round(S.timeLeft * 10) / 10, cuts: S.cuts, maxCombo: S.maxCombo, bombsHit: S.bombsHit,
    fruits: Wd.count(Wd.fruits), halves: Wd.count(Wd.halves), drops: Wd.count(Wd.drops), trail: Blade.count(S.clock),
    draws: lastDraws, muted: P.muted, clock: Math.round(S.clock * 10) / 10
  };
};

// 无头测试用：--eval 里拿到同一份世界（eval 里 require 会得到另一份模块实例）
GameGlobal.__world = Wd;

B.loop(update, render);
