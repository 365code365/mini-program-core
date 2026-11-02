// 星际战机（竖屏）：按住拖动战机，自动开火；三种敌机（侦察机 / 战斗机 / 轰炸机）+ 每 5 波一个首领，波次无尽。
// 击落掉道具：双（火力升级，最多 3 级，14 秒降一级）、盾（护盾 8 秒，挡弹、撞机）、炸（炸弹，最多 3 颗，左下角按钮 / 空格）。
// 三条命，被击中掉一级火力、2.4 秒无敌。PC：方向键 / WASD 移动，空格放炸弹，P 暂停。
// 画面走 WebGL（js/gl2d.js：精灵 + 烘焙小图 stamp，几百颗子弹 / 粒子一批画完）；对象全部预先建池（js/world.js），帧里不分配。
// 背景音乐 InnerAudioContext 循环（audio/bgm.wav），音效 WebAudio 预合成（js/base.js）；切后台暂停，回前台倒数 3 秒继续。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var Wd = require('js/world.js');
var ST = require('js/step.js');
var AI = require('js/ai.js');
var Art = require('js/art.js');
var Rn = require('js/render.js');
var W = B.W, H = B.H, S = Wd.S, UI = Rn.UI;
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);
var A = Art.make(R, W, H);
var view = Rn.make(R, A);

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';
var START_WAVE = Math.max(1, parseInt(q.wave, 10) || 1);      // 调试：?wave=5 直接打首领
var bottom = Math.max(H - (B.safe.bottom || H), 0);
S.best = Number(B.load('shooter.best', 0)) || 0;
var P = { paused: false, countdown: 0, hidden: false, muted: B.load('shooter.muted', false) === true };
B.setMuted(P.muted);
var lastDraws = 0;

// ───────── 声音：BGM 走 InnerAudioContext，音效走 WebAudio ─────────
var bgm = null;
try {
  bgm = wx.createInnerAudioContext();
  bgm.src = 'audio/bgm.wav'; bgm.loop = true; bgm.volume = 0.35;
  bgm.onError && bgm.onError(function (e) { console.log('[shooter] BGM 不可用：' + (e && e.errMsg)); bgm = null; });
} catch (e) { bgm = null; }
function music(on) {
  if (!bgm) return;
  try { if (on && !P.muted) bgm.play(); else bgm.pause(); } catch (e) {}
}
function delay(f, ms) { setTimeout(f, ms); }
Wd.setFx(function (name) {
  if (name === 'boom') { B.beep(0, 0.22, 'noise', 0.1); B.beep(140, 0.18, 'sine', 0.14, 0.4); }
  else if (name === 'boomBig') { B.beep(0, 0.5, 'noise', 0.16); B.beep(90, 0.4, 'sawtooth', 0.1, 0.35); }
  else if (name === 'bossDown') { B.beep(0, 0.9, 'noise', 0.2); [523, 659, 784, 1047].forEach(function (f, i) { delay(function () { B.beep(f, 0.12, 'triangle', 0.12); }, 200 + i * 90); }); }
  else if (name === 'hurt') B.beep(320, 0.35, 'sawtooth', 0.14, 0.3);
  else if (name === 'power') [660, 880, 1320].forEach(function (f, i) { delay(function () { B.beep(f, 0.07, 'triangle', 0.11); }, i * 55); });
  else if (name === 'bomb') { B.beep(0, 0.8, 'noise', 0.2); B.beep(60, 0.6, 'sine', 0.2, 0.5); }
  else if (name === 'warn') [440, 330, 440, 330].forEach(function (f, i) { delay(function () { B.beep(f, 0.18, 'square', 0.06); }, i * 260); });
  else if (name === 'over') B.beep(220, 0.7, 'sawtooth', 0.14, 0.3);
});

// ───────── 状态切换 ─────────
function start() { Wd.reset(START_WAVE); P.paused = false; P.countdown = 0; music(true); }
function toTitle() { S.state = 'ready'; music(false); }
function pause() { if (S.state === 'play' && !P.paused) { P.paused = true; P.countdown = 0; music(false); } }
function resume() { P.paused = false; P.countdown = 0; P.hidden = false; music(true); }
function setMuted(m) {
  P.muted = m; B.setMuted(m); B.save('shooter.muted', m);
  if (m) music(false);
}
B.onHide(function () { if (S.state === 'play' && !P.paused) { P.paused = true; P.hidden = true; } music(false); });
// 回前台：切后台前在玩的，倒数 3 秒自动继续（点「继续」可以跳过）
B.onShow(function () { if (P.paused && P.hidden) { P.countdown = 3; P.hidden = false; } });

// ───────── 输入：拖动是相对位移（手指不挡战机），按钮在按下时判 ─────────
var drag = { id: -1, x0: 0, y0: 0, sx0: 0, sy0: 0 };
B.on('down', function (x, y, id) {
  if (S.state !== 'play' || P.paused) return;
  if (B.hit(UI.pause, x, y)) { pause(); return; }
  if (B.hit(UI.bomb, x, y)) { Wd.bomb(); return; }
  if (drag.id === -1) { drag.id = id; drag.x0 = x; drag.y0 = y; drag.sx0 = S.sx; drag.sy0 = S.sy; }
});
B.on('move', function (x, y, id) {
  if (id !== drag.id || S.state !== 'play' || P.paused) return;
  S.sx = B.clamp(drag.sx0 + (x - drag.x0) * 1.2, 20, W - 20);
  S.sy = B.clamp(drag.sy0 + (y - drag.y0) * 1.2, H * 0.3, H - 40 - bottom);
});
B.on('up', function (x, y, id) { if (id === drag.id) drag.id = -1; });
B.on('tap', function (x, y) {
  if (S.state === 'ready') { if (B.hit(UI.a, x, y)) start(); }
  else if (S.state === 'over') { if (B.hit(UI.a, x, y)) start(); else if (B.hit(UI.b, x, y)) toTitle(); }
  else if (P.paused) {
    if (B.hit(UI.a, x, y)) resume();
    else if (B.hit(UI.b, x, y)) setMuted(!P.muted);
    else if (B.hit(UI.c, x, y)) { P.paused = false; Wd.over(); music(false); }
  }
});
B.on('key', function (k) {
  if (k === 'pause') { if (P.paused) resume(); else pause(); return; }
  if (k !== 'ok') return;
  if (S.state === 'ready' || (S.state === 'over' && S.clock - S.overAt > 0.6)) start();
  else if (P.paused) resume();
  else Wd.bomb();
});

// ───────── 主循环 ─────────
function update(dt) {
  if (P.paused) {
    S.clock += dt;                                   // 面板上的倒数、按钮出现要走时钟；世界不动
    if (P.countdown > 0 && (P.countdown -= dt) <= 0) resume();
    return;
  }
  if (S.state === 'play') {
    var kx = (B.held.right ? 1 : 0) - (B.held.left ? 1 : 0), ky = (B.held.down ? 1 : 0) - (B.held.up ? 1 : 0);
    if (kx || ky) {
      S.sx = B.clamp(S.sx + kx * 320 * dt, 20, W - 20);
      S.sy = B.clamp(S.sy + ky * 320 * dt, H * 0.3, H - 40 - bottom);
    }
    if (AUTO) {
      var o = AI.think(S, Wd, W, H, dt), dx = o.x - S.sx, dy = o.y - S.sy, d = Math.sqrt(dx * dx + dy * dy), sp = 380 * dt;
      if (d > sp) { dx *= sp / d; dy *= sp / d; }
      S.sx += dx; S.sy += dy;
      if (o.bomb) Wd.bomb();
    }
  }
  ST.step(dt);
  if (AUTO && S.state === 'ready') start();
  if (AUTO && S.state === 'over' && S.clock - S.overAt > 1.5) start();
}
function render() { lastDraws = view.draw(P); }

GameGlobal.__play = function () {
  var b = S.boss;
  return {
    state: S.state, paused: P.paused, score: S.score, best: S.best, wave: S.wave, lives: S.lives, bombs: S.bombs,
    power: S.power, shield: S.shieldT > 0, kills: S.kills, enemies: Wd.count(Wd.en), enemyBullets: Wd.count(Wd.eb),
    playerBullets: Wd.count(Wd.pb), particles: Wd.count(Wd.pt), boss: b.on ? { hp: b.hp, max: b.max } : null,
    draws: lastDraws, bgm: !!bgm, muted: P.muted, clock: Math.round(S.clock * 10) / 10
  };
};

S.state = 'ready';
if (AUTO) start();
B.loop(update, render);
