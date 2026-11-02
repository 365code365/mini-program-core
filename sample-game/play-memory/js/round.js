// 一局翻牌：发牌、翻开 / 盖回、配对判定、计时、计步、连对加分、星级。只管状态与补间，不画。
// 牌 { i, item, f: 翻面进度 0 背 → 1 面, up: 目标是否朝上, matched, s: 缩放, x, y: 左上角, w, h, hx, hy: 原位 }
var T = require('tween.js');

var LEVELS = [
  { id: 'easy', name: '初级', cols: 4, rows: 4 },
  { id: 'mid', name: '中级', cols: 4, rows: 5 },
  { id: 'hard', name: '高级', cols: 6, rows: 6 }
];

function shuffle(a) {
  for (var i = a.length - 1; i > 0; i--) { var j = Math.floor(Math.random() * (i + 1)), t = a[i]; a[i] = a[j]; a[j] = t; }
  return a;
}

// box = 牌区 { x, y, w, h }；snd = { flip(), match(combo), miss(), win() }
function Round(level, theme, box, snd) {
  this.level = level; this.theme = theme; this.snd = snd;
  this.pairs = level.cols * level.rows / 2;
  var gap = level.cols >= 6 ? 6 : 10;
  var cw = Math.floor((box.w - gap * (level.cols - 1)) / level.cols);
  var ch = Math.min(Math.floor(cw * 1.3), Math.floor((box.h - gap * (level.rows - 1)) / level.rows));
  var ox = box.x + Math.floor((box.w - cw * level.cols - gap * (level.cols - 1)) / 2);
  var oy = box.y + Math.floor((box.h - ch * level.rows - gap * (level.rows - 1)) / 2);
  var picks = shuffle(theme.items.slice()).slice(0, this.pairs);
  var deck = shuffle(picks.concat(picks));
  var cx = box.x + box.w / 2 - cw / 2, cy = box.y + box.h + 40;
  this.cards = deck.map(function (it, i) {
    var c = i % level.cols, r = Math.floor(i / level.cols);
    var card = { i: i, id: i + 1, item: it, f: 0, up: false, matched: false, s: 1, w: cw, h: ch,
      hx: ox + c * (cw + gap), hy: oy + r * (ch + gap), x: cx, y: cy };
    // 发牌：从下面中间一张张飞到位
    T.to(card, { x: card.hx, y: card.hy }, 0.35, 'outQuad', i * 0.025);
    return card;
  });
  this.open = []; this.pending = null;
  this.steps = 0; this.score = 0; this.combo = 0; this.maxCombo = 0; this.time = 0;
  this.started = false; this.matchedPairs = 0; this.result = null; this.misses = 0;
}
var P = Round.prototype;
P.LEVELS = LEVELS;

P.flipUp = function (c) { c.up = true; T.kill(c); T.to(c, { f: 1 }, 0.28, 'inOutQuad'); this.snd.flip(); };
P.flipDown = function (c) { c.up = false; T.kill(c); T.to(c, { f: 0, s: 1 }, 0.28, 'inOutQuad'); };

// 点了第 i 张（返回是否翻开了）
P.tap = function (i) {
  var c = this.cards[i];
  if (!c || this.result || c.matched || c.up) return false;
  if (this.open.length === 2) this.settle();               // 前两张还没盖回就点了第三张：立刻结算前两张
  this.started = true;
  this.flipUp(c);
  this.open.push(c);
  if (this.open.length === 2) {
    this.steps++;
    var same = this.open[0].item.key === this.open[1].item.key;
    this.pending = { t: same ? 0.32 : 0.9, same: same };
  }
  return true;
};

// 结算翻开的两张：配上了就定住 + 连对加分；没配上就盖回、连对清零
P.settle = function () {
  var p = this.pending, a = this.open[0], b = this.open[1];
  this.pending = null; this.open = [];
  if (!p) return;
  if (p.same) {
    a.matched = b.matched = true;
    this.combo++; this.maxCombo = Math.max(this.maxCombo, this.combo);
    this.score += 100 + 50 * (this.combo - 1);              // 连对：100、150、200…
    this.matchedPairs++;
    [a, b].forEach(function (c) { T.kill(c); c.f = 1; c.s = 1.14; T.to(c, { s: 1 }, 0.3, 'outBack'); });
    this.snd.match(this.combo);
    if (this.matchedPairs === this.pairs) this.finish();
  } else {
    this.misses++;
    this.combo = 0;
    this.flipDown(a); this.flipDown(b);
    this.snd.miss();
  }
};

P.update = function (dt) {
  if (this.started && !this.result) this.time += dt;
  if (this.pending) { this.pending.t -= dt; if (this.pending.t <= 0) this.settle(); }
};

// 全部配完：时间奖励（比标准用时快多少秒 × 10），步数定星级
P.finish = function () {
  var par = this.pairs * 3.5, bonus = Math.max(0, Math.round(par - this.time)) * 10;
  this.score += bonus;
  var stars = this.steps <= this.pairs * 1.5 ? 3 : this.steps <= this.pairs * 2.2 ? 2 : 1;
  this.result = { score: this.score, bonus: bonus, steps: this.steps, time: Math.round(this.time * 10) / 10, stars: stars, maxCombo: this.maxCombo };
  this.snd.win();
};

P.cardAt = function (x, y) {
  for (var i = 0; i < this.cards.length; i++) {
    var c = this.cards[i];
    if (x >= c.hx && x <= c.hx + c.w && y >= c.hy && y <= c.hy + c.h) return i;
  }
  return -1;
};

module.exports = Round;
module.exports.LEVELS = LEVELS;
