// 拳魂：横屏 2D 格斗（单人 vs 电脑，三局两胜）。原创角色，玩法照 90 年代街机格斗：拳脚轻重、搓招必杀、能量超必杀、连段、防御、击倒。
// 画面走 WebGL（js/gl2d.js，带贴图）；角色、场景、封面是 AI 美术（tools/gen-fighter-art.py），没加载到就退回代码画的剪影。
//
// 时间：逻辑固定 60 步 / 秒（帧数据、判定、电脑反应都按「帧」写，和屏幕刷新率无关：144Hz 屏上一帧逻辑画 2.4 次），
// 画面每个 rAF 画一次。KO 慢放 = 逻辑步长按 fight.slow 缩小。
var NOW = typeof performance !== 'undefined' && performance.now ? function () { return performance.now(); } : Date.now;   // 计时用（Date.now 只到毫秒）
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var D = require('js/data.js');
var Art = require('js/art.js');
var Fight = require('js/fight.js');
var C = require('js/controls.js');
var HUD = require('js/hud.js');
var Render = require('js/render.js');
var S = require('js/scenes.js');
var Audio = require('js/audio.js');

var W = B.W, H = B.H;
var g = G2.make(B.canvas, W, H, B.sys.pixelRatio || 2);
var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

var art = { chars: {}, stages: {}, cover: null };
var scene = 'loading', sceneT = 0, loadK = 0, clock = 0;
var st = { diff: Number(B.load('fighter.diff', 1)) || 0, muted: !!B.load('fighter.muted', false), pick: 'yan', cpu: 'shuang', stage: 0 };
if (st.diff > 2) st.diff = 1;
if (q.stage) st.stage = Number(q.stage) % D.STAGES.length;
B.setMuted(st.muted);
Audio.setMuted(st.muted);
var pendingScene = null, fight = null, result = null, paused = false, hits = {}, acc = 0, matches = 0;

// ───────── 音效：sfx/*.mp3（tools/gen-fighter-sfx.py 离线合成），js/audio.js 解码一次、每次一个 BufferSource ─────────
// 逻辑里的事件名 → 音效文件名（同一个事件按角色分：焰的波是火、霜的波是冰电，在 fight.js 里已经按 m.sfx 区分）
var ALIAS = { round: null, fight: 'v_fight', win: 'v_win', lose: 'v_lose', light: 'hitL', heavy: 'hitH' };
function sfx(n, opt) {
  if (AUTO && q.sound !== '1') return;            // 自对战默认静音；sound=1 打开（真机量「放音效会不会掉帧」用）
  var f = ALIAS.hasOwnProperty(n) ? ALIAS[n] : n;
  if (f) Audio.play(f, opt);
  if (n === 'hitH' || n === 'ko' || n === 'super') B.buzz();
}

function go(s) { scene = s; sceneT = 0; C.setEnabled(s === 'fight'); }

function startFight() {
  matches++;
  fight = new Fight({
    p1: st.pick, p2: st.cpu, art: art, stage: D.STAGES[st.stage], diff: st.diff, auto: AUTO, cpuOff: q.cpu === 'off', W: W, H: H, sfx: sfx,
    onEnd: function (r) { result = r; go('result'); if (AUTO) setTimeout(function () { randomize(); go('vs'); }, 1000); }
  });
  lf = 0;
  if (q.meter) fight.p[0].meter = Number(q.meter);      // 调试：开局就给能量（无头测超必杀）
  if (q.wins) fight.wins = [Number(q.wins), 0];            // 调试：预置回合胜场（无头测结算画面）
  if (q.hp) fight.p[1].hp = fight.p[1].shown = fight.p[1].trail = Number(q.hp);   // 调试：对手开局血量（无头测 KO / 结算）
  paused = false; acc = 0;
  go('fight');
}
function randomize() {
  st.pick = D.ORDER[Math.floor(Math.random() * 2)]; st.cpu = D.ORDER[Math.floor(Math.random() * 2)];
  if (st.cpu === st.pick) st.cpu = D.ORDER[(D.ORDER.indexOf(st.pick) + 1) % 2];
  st.stage = Math.floor(Math.random() * D.STAGES.length);
}

// 美术与音效一起加载（进度条按美术走：音效解码快）
var sfxLoaded = 0;
Promise.all([Art.load(g, function (k) { loadK = k; }), Audio.init()]).then(function (r) {
  var a = r[0]; sfxLoaded = r[1];
  art = a;
  if (AUTO) { randomize(); startFight(); } else go('title');
  if (pendingScene || q.scene) GameGlobal.__goto(pendingScene || q.scene);
});

// ───────── 点按（菜单、暂停键） ─────────
var PAUSE = { x: W / 2 - 18, y: HUD.TOP + 46, w: 36, h: 26 };
B.on('down', function (x, y) {
  var h = hits;
  if (scene === 'title') {
    if (B.hit(h.start, x, y)) { sfx('select'); go('select'); }
    else if (B.hit(h.diff, x, y)) { st.diff = (st.diff + 1) % 3; B.save('fighter.diff', st.diff); sfx('ui'); }
    else if (B.hit(h.sound, x, y)) { st.muted = !st.muted; B.setMuted(st.muted); Audio.setMuted(st.muted);
Audio.setMuted(st.muted); B.save('fighter.muted', st.muted); sfx('ui'); }
  } else if (scene === 'select') {
    D.ORDER.forEach(function (id) { if (B.hit(h[id], x, y)) { st.pick = id; sfx('select'); } });
    if (B.hit(h.stage, x, y)) { st.stage = (st.stage + 1) % D.STAGES.length; sfx('ui'); }
    if (B.hit(h.go, x, y)) { st.cpu = D.ORDER[(D.ORDER.indexOf(st.pick) + 1) % 2]; sfx('superStart', { vol: 0.6 }); go('vs'); }
    if (B.hit(h.back, x, y)) go('title');
  } else if (scene === 'fight') {
    if (!paused && B.hit(PAUSE, x, y)) { paused = true; C.setEnabled(false); }
    else if (paused) {
      if (B.hit(h.resume, x, y)) { paused = false; C.setEnabled(true); }
      else if (B.hit(h.restart, x, y)) startFight();
      else if (B.hit(h.quit, x, y)) { paused = false; go('title'); }
    }
  } else if (scene === 'result' && sceneT > 0.6) {
    if (B.hit(h.again, x, y)) go('vs');
    else if (B.hit(h.select, x, y)) go('select');
    else if (B.hit(h.quit, x, y)) go('title');
  }
});
B.on('key', function (k) {
  if (k === 'pause' && scene === 'fight') { paused = !paused; C.setEnabled(!paused); }
  if (k === 'ok' && scene === 'title') go('select');
});
B.onShow(function () { Audio.resume(); });
B.onHide(function () { if (scene === 'fight' && !AUTO) { paused = true; C.setEnabled(false); } });

// ───────── 主循环 ─────────
var perf = { logic: 0, draw: 0, draws: 0 }, perfN = 0;
// 调试：demo=130:su,200:lp —— 第 N 个逻辑帧（从开战算）替玩家按下某个键，无头回归里按确定的帧出招
var DEMO = {};
(q.demo || '').split(',').forEach(function (s) { var kv = s.split(':'); if (kv.length === 2) DEMO[Number(kv[0])] = kv[1]; });
var lf = 0;
function input() {
  var inp = C.sample();
  lf++;
  if (DEMO[lf]) { var k = DEMO[lf]; if (k === 'R' || k === 'L') inp.x = k === 'R' ? 1 : -1; else inp.press[k] = true; }
  return inp;
}
function update(dt) {
  clock += dt; sceneT += dt;
  C.step(dt);
  if (scene === 'vs' && sceneT > 1.7) startFight();
  if ((scene === 'fight' || scene === 'result') && fight && !paused) {
    var t0 = NOW();
    acc += dt * 60 * fight.slow;
    var n = 0;
    while (acc >= 1 && n < 4) { fight.step(scene === 'fight' ? input() : { x: 0, y: 0, press: {} }); acc -= 1; n++; }
    if (acc > 4) acc = 0;
    perf.logic = NOW() - t0;
  }
}

function render() {
  var t0 = NOW();
  g.begin('#000', !!(fight && (scene === 'fight' || scene === 'result') && art.stages[fight.stage.id]));
  perf.begin = NOW() - t0;
  hits = {};
  if (scene === 'loading') S.loading(g, art, loadK, clock);
  else if (scene === 'title') hits = S.titleScreen(g, art, st, clock);
  else if (scene === 'select') hits = S.selectScreen(g, art, st, clock);
  else if (scene === 'vs') S.vsScreen(g, art, st, sceneT);
  else if (fight) {
    Render.render(g, fight, W, H, { boxes: q.boxes === '1', perf: perf });
    var th = NOW();
    HUD.draw(g, fight, fight.pt);
    perf.hud = NOW() - th; th = NOW();
    if (scene === 'fight') {
      var showCtl = fight.phase === 'fight' || fight.phase === 'intro';
      // 超必杀演出时按键淡掉，不挡切入画面
      C.draw(g, fight.p[0].stocks(), AUTO ? 0 : fight.freeze > 0 ? 0.15 : showCtl ? 1 : 0.35);
      if (!AUTO) {
        g.rrect(PAUSE.x, PAUSE.y, PAUSE.w, PAUSE.h, 6, 'rgba(0,0,0,0.45)');
        g.rect(W / 2 - 6, PAUSE.y + 7, 4, 12, '#ffffff'); g.rect(W / 2 + 2, PAUSE.y + 7, 4, 12, '#ffffff');
      }
      perf.ctl = NOW() - th;
      if (paused) hits = S.pauseScreen(g, clock);
    } else if (scene === 'result' && result) hits = S.resultScreen(g, result, sceneT, art, st.pick);
  }
  var te = NOW();
  perf.draws = g.end();
  perf.flush = NOW() - te;
  perf.draw = NOW() - t0;
  perf.verts = g.verts();
  // 调试：perf=1 每秒打一行各段耗时（真机定位热点用）
  if (fight && q.perf === '1') perf.p1 = [fight.p[0].state, fight.p[0].lastKey || '', fight.p[0].moveCount || 0, Math.round(fight.p[0].x)];
  if (q.perf === '1' && ++perfN % 20 === 0) perf.sfx = sfxLoaded;
    console.log('[perf] ' + JSON.stringify(perf, function (k, v) { return typeof v === 'number' ? Math.round(v * 10) / 10 : v; }));
}

B.loop(update, render);

// 无头 / 演示：auto=1 两边都是电脑（玩家这边用困难难度），一场打完一秒后随机换人换场景再来
GameGlobal.__play = function () {
  return {
    scene: scene, matches: matches, phase: fight && fight.phase, round: fight && fight.round, wins: fight && fight.wins,
    hp: fight && [fight.p[0].hp, fight.p[1].hp], meter: fight && [fight.p[0].meter, fight.p[1].meter],
    states: fight && [fight.p[0].state, fight.p[1].state], timer: fight && Math.ceil(fight.timer / 60),
    maxCombo: fight && fight.stats.maxCombo, sfx: sfxLoaded, art: Object.keys(art.chars).concat(Object.keys(art.stages)), perf: perf
  };
};
// 调试：直接跳到某个画面（无头截图用）。scene = title / select / vs / fight
GameGlobal.__fight = function () { return fight; };
// 还在加载时先记下，加载完再跳（否则加载完成的回调又切回标题）；启动参数 scene=… 同理
GameGlobal.__goto = function (s) {
  if (scene === 'loading') { pendingScene = s; return 'pending:' + s; }
  if (s === 'fight') startFight(); else go(s);
  return scene;
};
