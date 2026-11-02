// 记忆翻牌（竖屏）：初级 4×4 / 中级 4×5 / 高级 6×6。每次翻两张，一样就配上，不一样一秒后盖回。
// 计时、计步，连着配上分数递增（100、150、200…），比标准用时快有速度奖励，步数定星级。
// 卡面三套：几何（主包）、节气 / 生肖（分包 themes/，选到时 wx.loadSubpackage）。背景音乐 InnerAudioContext 循环，切后台暂停。
var B = require('js/base.js');
var G2 = require('js/gl2d.js');
var T = require('js/tween.js');
var Themes = require('js/themes.js');
var Round = require('js/round.js');
var View = require('js/view.js');
var Bgm = require('js/bgm.js');
var W = B.W, H = B.H, dpr = B.sys.pixelRatio || 2;
var R = G2.make(B.canvas, W, H, dpr);

var q = (wx.getLaunchOptionsSync && wx.getLaunchOptionsSync().query) || {};
var AUTO = q.auto === '1';

var top = Math.max(B.hudTop, 44);
var btnY = Math.min(H - 36, B.safe.bottom - 30);
var box = { x: 12, y: top + 92, w: W - 24, h: btnY - 30 - (top + 92) };
var view = View.make(R, W, H, top);

// ───────── 存档 ─────────
var best = B.load('mem.best', {});
if (!best || typeof best !== 'object') best = {};
var S = { best: best, theme: B.load('mem.theme', 'geo'), music: B.load('mem.music', true) !== false, toast: '', newBest: false,
  resultAge: 0, btnY: btnY };
if (!Themes.META.some(function (m) { return m.id === S.theme; })) S.theme = 'geo';

// ───────── 音效（WebAudio 预合成）与背景音乐 ─────────
var PENTA = [0, 2, 4, 7, 9, 12, 14, 16, 19, 21];
var snd = {
  flip: function () { B.beep(900, 0.04, 'triangle', 0.06, 0.8); },
  match: function (combo) {
    var f = 523 * Math.pow(2, PENTA[Math.min(PENTA.length - 1, combo - 1)] / 12);
    B.beep(f, 0.12, 'sine', 0.12); setTimeout(function () { B.beep(f * 1.5, 0.14, 'sine', 0.1); }, 70);
  },
  miss: function () { B.beep(220, 0.14, 'square', 0.05, 0.8); },
  win: function () { [523, 659, 784, 1046, 1318].forEach(function (f, i) { setTimeout(function () { B.beep(f, 0.12, 'triangle', 0.11); }, i * 90); }); }
};
Bgm.init(S.music);

var screen = 'menu', round = null, lastLevel = 'easy', played = 0, resultDelay = 0, autoT = 0, autoGame = 0;

function levelById(id) { return Round.LEVELS.filter(function (l) { return l.id === id; })[0] || Round.LEVELS[0]; }

var A = {
  play: function (id) {
    lastLevel = id || lastLevel;
    Themes.ensure(S.theme, function (theme, err) {
      if (err) { S.toast = '「' + S.theme + '」主题加载失败，先用几何：' + err; S.theme = 'geo'; }
      T.clear();
      round = new Round(levelById(lastLevel), theme, box, snd);
      screen = 'play'; resultDelay = 0; S.newBest = false;
    });
  },
  retry: function () { A.play(lastLevel); },
  menu: function () { screen = 'menu'; },
  theme: function (id) {
    S.toast = '';
    Themes.ensure(id, function (theme, err) {
      if (err) { S.toast = '主题加载失败：' + err; return; }
      S.theme = theme.id; B.save('mem.theme', S.theme);
    });
  },
  music: function () { S.music = !S.music; Bgm.set(S.music); B.save('mem.music', S.music); }
};

function finish() {
  var res = round.result, id = round.level.id, old = best[id];
  if (!old || res.score > old.score) { best[id] = { score: res.score, steps: res.steps, time: res.time, stars: res.stars, theme: round.theme.id }; S.newBest = true; }
  B.save('mem.best', best);
  played++;
  screen = 'result'; S.resultAge = 0;
}

// ───────── 输入 ─────────
B.on('tap', function (x, y) {
  for (var i = view.btns.length - 1; i >= 0; i--) {
    var b = view.btns[i];
    if (B.hit(b, x, y)) { B.beep(660, 0.04, 'sine', 0.06); b.fn(b.arg); return; }
  }
  if (screen === 'play' && round && !round.result) {
    var k = round.cardAt(x, y);
    if (k >= 0) { round.tap(k); if (AUTO) remember(k); }
  }
});
B.on('key', function (k) {
  if (k === 'ok' && screen !== 'play') A.play(lastLevel);
  if (k === 'pause' && screen === 'play') A.menu();
});

// ───────── 自动试玩：记住翻开过的牌，有已知的一对就翻那一对，否则翻没见过的 ─────────
// 依次玩 初级·节气 → 中级·生肖 → 高级·几何（顺带把分包加载走一遍）
var PLAN = [['easy', 'jieqi'], ['mid', 'zodiac'], ['hard', 'geo']];
var seen = {};
function remember(i) { seen[i] = round.cards[i].item.key; }
function think() {
  var cards = round.cards, open = round.open.length === 2 ? [] : round.open, i, j;
  function free(k) { return !cards[k].matched && !cards[k].up; }
  if (open.length === 1) {
    var key = open[0].item.key;
    for (i in seen) if (seen[i] === key && free(+i)) return +i;
  } else {
    for (i in seen) for (j in seen) if (i < j && seen[i] === seen[j] && free(+i) && free(+j)) return +i;
  }
  var unseen = [];
  for (i = 0; i < cards.length; i++) if (free(i) && seen[i] === undefined) unseen.push(i);
  if (!unseen.length) for (i = 0; i < cards.length; i++) if (free(i)) unseen.push(i);
  return unseen.length ? unseen[Math.floor(Math.random() * unseen.length)] : -1;
}
function auto(dt) {
  autoT += dt;
  if (screen === 'menu' && autoT > 0.6 && !Themes.loading()) {
    var p = PLAN[autoGame % PLAN.length]; autoGame++; autoT = 0;
    seen = {};
    S.theme = p[1];
    A.play(p[0]);
  } else if (screen === 'play' && !round.result && autoT > 0.32 && !T.busy()) {
    autoT = 0;
    var k = think();
    if (k >= 0 && round.tap(k)) remember(k);
  } else if (screen === 'result' && autoT > 1.6 && q.hold !== '1') { autoT = 0; screen = 'menu'; }   // &hold=1：停在结算（截图用）
}

function update(dt) {
  T.update(dt);
  if (screen === 'play' && round) {
    round.update(dt);
    if (round.result) { resultDelay += dt; if (resultDelay > 0.6) finish(); }
  }
  if (screen === 'result') S.resultAge += dt;
  if (AUTO) auto(dt);
}

function render() {
  R.begin('#e0f7fa');
  view.reset();
  view.background();
  if (screen === 'menu') view.menu(S, A);
  else if (round) {
    view.cards(round);
    view.hud(S, round, A);
    if (screen === 'result') view.result(S, round, A);
  }
  R.end();
}

GameGlobal.__play = function () {
  return {
    screen: screen, played: played, theme: S.theme, themesLoaded: Themes.META.filter(function (m) { return Themes.loaded(m.id); }).map(function (m) { return m.id; }),
    level: round ? round.level.id : null, pairs: round ? round.pairs : 0, matched: round ? round.matchedPairs : 0,
    steps: round ? round.steps : 0, score: round ? round.score : 0, combo: round ? round.combo : 0, maxCombo: round ? round.maxCombo : 0,
    time: round ? Math.round(round.time * 10) / 10 : 0, result: round ? round.result : null, best: best, bgm: Bgm.state()
  };
};

B.loop(update, render);
