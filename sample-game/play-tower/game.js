// 塔防守卫（竖屏）：敌人沿固定路线（BFS 寻出来的，js/maps.js）走向城堡，点空地建塔（箭 / 炮 / 冰 / 电，各 3 级），
// 守满 10 波过关；漏一个扣 1 点生命（Boss 扣 5），生命归零失败。3 张关卡图依次解锁，星级与最高分存 storage。
// 索敌 / 范围 / 伤害走 WXWebAssembly（wasm/combat.wasm），分桶在 js/combat.js；画面走 WebGL（js/gl2d.js）。
// PC：空格出兵、P 暂停。启动参数 auto=1：自动试玩（js/auto.js，3 倍速）。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var Maps = require('js/maps.js');
var D = require('js/data.js');
var Combat = require('js/combat.js');
var Sim = require('js/sim.js');
var Draw = require('js/draw.js');
var Ui = require('js/ui.js');
var Auto = require('js/auto.js');
var W = B.W, H = B.H;
var R = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2, { antialias: false });   // 为什么关 MSAA 见 js/gl2d.js 的 make

// 版面：顶栏 → 9×12 的地图 → 底部面板。格子按宽高两头取小
var OY = B.hudTop + 40;
var CELL = Math.floor(Math.min((W - 16) / Maps.COLS, (H - OY - 104) / Maps.ROWS));
var OX = Math.floor((W - CELL * Maps.COLS) / 2);
var L = { OX: OX, OY: OY, CELL: CELL };

var combat = Combat.make(Maps.COLS, Maps.ROWS);
var sim = Sim.make(combat), S = sim.S;
var draw = Draw.make(R, L);
var ui = Ui.make(R, L, draw);
var auto = Auto.make(sim);

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';
var STEP = 1 / 60;

// 进度：{ unlocked: 已解锁到第几关, best: { 关号: { score, stars } } }
var P = B.load('tower.progress', null);
if (!P || typeof P !== 'object' || !P.best) P = { unlocked: 1, best: {} };
P.unlocked = Math.max(1, Math.min(Maps.list.length, Number(P.unlocked) || 1));

var G = { screen: 'title', speed: 1, paused: false, sel: null, clock: 0, acc: 0, endAt: 0, newBest: false, wasm: '', saved: false, games: 0 };
combat.init(function (mode) { G.wasm = mode; var i = combat.info(); if (i.note) console.log('[tower] 战斗计算用 JS：' + i.note); });

function start(idx) {
  sim.reset(idx); auto.reset();
  G.screen = 'play'; G.paused = false; G.sel = null; G.saved = false; G.newBest = false; G.games++;
  if (AUTO) G.speed = 3;
}
function finish() {
  if (G.saved) return;
  G.saved = true; G.endAt = G.clock;
  var key = S.mapIdx + 1, b = P.best[key];
  if (S.state === 'win') {
    G.newBest = !b || S.score > b.score;
    P.best[key] = { score: Math.max(S.score, b ? b.score : 0), stars: Math.max(S.stars, b ? b.stars : 0) };
    P.unlocked = Math.min(Maps.list.length, Math.max(P.unlocked, key + 1));
  } else if (b) G.newBest = false;
  B.save('tower.progress', P);
  try { S.state === 'lose' ? wx.vibrateLong({}) : wx.vibrateShort({ type: 'medium' }); } catch (e) {}
}

// ───────── 输入 ─────────
var OVERLAY = { resume: 1, restart: 1, menu: 1, next: 1 };
function press(b) {
  var ended = S.state === 'win' || S.state === 'lose';
  if ((G.paused || ended) && G.screen === 'play' && !OVERLAY[b.k]) return false;
  switch (b.k) {
    case 'start': start(b.arg); break;
    case 'speed': G.speed = G.speed >= 3 ? 1 : G.speed + 1; break;
    case 'pause': G.paused = true; break;
    case 'resume': G.paused = false; break;
    case 'restart': start(S.mapIdx); break;
    case 'menu': G.screen = 'title'; G.paused = false; break;
    case 'next': start(Math.min(Maps.list.length - 1, S.mapIdx + 1)); break;
    case 'wave': sim.callWave(); break;
    case 'build':
      if (!G.sel) break;
      var t = sim.build(G.sel.c, G.sel.r, b.arg);
      if (t) G.sel = { c: t.c, r: t.r, tower: t };
      else { toast(S.gold < D.TOWERS[b.arg].cost ? '金币不够' : '这里不能建'); return true; }
      break;
    case 'upgrade': if (G.sel && G.sel.tower && !sim.upgrade(G.sel.tower)) toast('金币不够'); break;
    case 'sell': if (G.sel && G.sel.tower) { sim.sell(G.sel.tower); G.sel = null; } break;
    default: return false;
  }
  B.beep(880, 0.03, 'square', 0.04);
  return true;
}
function toast(t) { try { wx.showToast({ title: t, icon: 'none', duration: 1000 }); } catch (e) {} }
B.on('tap', function (x, y) {
  var b = ui.hit(x, y);
  if (b && press(b)) return;
  if (G.screen !== 'play' || G.paused || S.state !== 'play') return;
  var c = Math.floor((x - OX) / CELL), r = Math.floor((y - OY) / CELL);
  if (c < 0 || r < 0 || c >= Maps.COLS || r >= Maps.ROWS) { G.sel = null; return; }
  var t = sim.towerAt(c, r);
  if (G.sel && G.sel.c === c && G.sel.r === r) G.sel = null;            // 再点一下取消
  else if (t) G.sel = { c: c, r: r, tower: t };
  else if (Maps.buildable(S.map, c, r)) G.sel = { c: c, r: r };
  else G.sel = null;
});
B.on('key', function (k) {
  if (G.screen === 'title') { if (k === 'ok') start(P.unlocked - 1); return; }
  if (k === 'pause') { if (S.state === 'play') G.paused = !G.paused; }
  else if (k === 'ok') { if (G.paused) G.paused = false; else if (S.state === 'play') sim.callWave(); else start(S.mapIdx); }
});
B.onHide(function () { if (G.screen === 'play' && S.state === 'play') G.paused = true; });

// ───────── 主循环：逻辑固定步长（每帧 dt × 倍速，按 1/60 切步，一帧最多 6 步防雪崩） ─────────
function update(dt) {
  G.clock += dt;
  if (AUTO) autoMeta();
  if (G.screen !== 'play') return;
  if (S.state !== 'play') { finish(); return; }
  if (G.paused) return;
  G.acc = Math.min(G.acc + dt * G.speed, STEP * 6);
  while (G.acc >= STEP) {
    G.acc -= STEP;
    if (AUTO) auto.step(S, STEP);
    sim.step(STEP);
    if (S.state !== 'play') break;
  }
  if (G.sel && G.sel.tower && S.towers.indexOf(G.sel.tower) < 0) G.sel = null;
}
// 自动试玩的「人」：标题页直接开第一张已解锁的图；结算后过 2 秒：赢了下一关、输了重来
function autoMeta() {
  if (G.screen === 'title' && G.clock > 0.5) start(Math.max(0, P.unlocked - 1));
  else if (G.screen === 'play' && S.state !== 'play' && G.saved && G.clock - G.endAt > 2) start(S.state === 'win' ? Math.min(Maps.list.length - 1, S.mapIdx + 1) : S.mapIdx);
}

function render() {
  if (G.screen === 'title') { R.begin('#0f1f14'); ui.draw(S, G, P, sim); R.end(); return; }
  R.begin('#24452b');
  draw.field(S, G.sel, G.clock);
  ui.draw(S, G, P, sim);
  G.draws = R.end();
}

GameGlobal.__play = function () {
  var i = combat.info();
  return { screen: G.screen, state: S.state, map: S.mapIdx + 1, wave: S.wave, lives: S.lives, gold: S.gold, kills: S.kills, leaked: S.leaked,
    towers: S.towers.length, levels: S.towers.reduce(function (a, t) { return a + t.lvl; }, 0), enemies: sim.aliveCount(), score: S.score,
    speed: G.speed, wasm: i.mode, wasmCalls: i.calls, unlocked: P.unlocked, games: G.games, draws: G.draws, cell: CELL };
};

B.loop(update, render);
