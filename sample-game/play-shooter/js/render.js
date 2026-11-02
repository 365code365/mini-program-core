// 绘制：背景图层 → 敌机 / 首领（精灵，连着画不打断批次）→ 道具、我方子弹、战机 → 敌弹、粒子（stamp，一批）→ HUD / 面板。
// 按钮命中框写进 UI，game.js 用它判点按。
var B = require('./base.js');
var Wd = require('./world.js');
var S = Wd.S, W = B.W, H = B.H;
var UI = { pause: null, bomb: null, a: null, b: null, c: null };
var PU_COLOR = ['#ffca28', '#26c6da', '#ef5350'], PU_CHAR = ['双', '盾', '炸'];
var POWER_S = ['', '', '火力 2 级', '火力 3 级'], BOMB_S = ['×0', '×1', '×2', '×3'];
var TINT = [1, 1, 1, 1], TINT2 = [1, 1, 1, 1];          // stamp 的染色用这两个数组改值，不每次新建
function tint(t, r, g, b, a) { t[0] = r; t[1] = g; t[2] = b; t[3] = a; return t; }

var cache = { score: -1, scoreS: '', best: -1, bestS: '', wave: -1, waveS: '' };
function strings() {
  if (cache.score !== S.score) { cache.score = S.score; cache.scoreS = String(S.score); }
  if (cache.best !== S.best) { cache.best = S.best; cache.bestS = '最高 ' + S.best; }
  if (cache.wave !== S.wave) { cache.wave = S.wave; cache.waveS = '第 ' + S.wave + ' 波'; }
}

function make(R, A) {
  var bottom = Math.max(B.H - (B.safe.bottom || B.H), 0);

  function world() {
    var i;
    for (i = 0; i < Wd.en.length; i++) { var e = Wd.en[i]; if (e.on) A.enemy(e.type, e.x, e.y, e.flash > 0); }
    var b = S.boss;
    if (b.on) A.boss(b.x, b.y, b.flash > 0);
    if (b.on) R.stamp(A.glow, b.x, b.y + 2, 1.6 + Math.sin(S.clock * 8) * 0.2, tint(TINT, 1, 0.8, 0.2, 0.9), true);
    for (i = 0; i < Wd.pu.length; i++) {
      var p = Wd.pu[i]; if (!p.on) continue;
      var s = 1 + Math.sin(p.t * 6) * 0.08;
      R.stamp(A.glow, p.x, p.y, 1.5 * s, R.color(PU_COLOR[p.k]), true);
      R.circle(p.x, p.y, 13 * s, PU_COLOR[p.k]);
      R.strokeCircle(p.x, p.y, 13 * s, 2, '#ffffff');
      R.text(PU_CHAR[p.k], p.x, p.y, 13, '#ffffff', 'center', 'bold');
    }
    for (i = 0; i < Wd.pb.length; i++) { var q = Wd.pb[i]; if (q.on) R.stamp(A.pshot, q.x, q.y, 1, null, true); }
    if (S.state === 'play' || S.state === 'ready') ship();
    for (i = 0; i < Wd.eb.length; i++) { var z = Wd.eb[i]; if (z.on) R.stamp(A.ebTex[z.k], z.x, z.y, 1); }
    for (i = 0; i < Wd.pt.length; i++) {
      var t = Wd.pt[i]; if (!t.on) continue;
      var k = t.life / t.max;
      R.stamp(A.glow, t.x, t.y, t.s * (0.35 + 0.65 * k), tint(TINT, t.r, t.g, t.b, k), true);
    }
    for (i = 0; i < Wd.rings.length; i++) {
      var g = Wd.rings[i]; if (!g.on) continue;
      var f = g.t / g.max;
      g.c[3] = (1 - f) * 0.8;
      R.strokeCircle(g.x, g.y, 6 + g.r1 * f, 3 * (1 - f) + 1, g.c);
    }
    for (i = 0; i < Wd.texts.length; i++) {
      var x = Wd.texts[i]; if (!x.on) continue;
      R.setAlpha(Math.min(1, (0.9 - x.t) * 3)); R.text(x.s, x.x, x.y, 13, x.c, 'center', 'bold'); R.setAlpha(1);
    }
  }
  function ship() {
    var flick = 0.8 + Math.sin(S.clock * 50) * 0.2;
    R.stamp(A.glow, S.sx - 4, S.sy + 22, 0.6 * flick, tint(TINT, 1, 0.6, 0.2, 1), true);
    R.stamp(A.glow, S.sx + 4, S.sy + 22, 0.6 * flick, TINT, true);
    var blink = S.invT > 0 && Math.floor(S.invT * 10) % 2 === 0;
    A.ship(S.sx, S.sy, 1, blink ? 0.35 : 1);
    if (S.shieldT > 0) {
      var a = S.shieldT < 2 ? (Math.floor(S.shieldT * 8) % 2 ? 0.3 : 0.8) : 0.8;
      R.stamp(A.glow, S.sx, S.sy, 2.2, tint(TINT2, 0.2, 0.8, 1, a * 0.35), true);
      R.strokeCircle(S.sx, S.sy, 30 + Math.sin(S.clock * 6), 2, tint(TINT2, 0.5, 0.95, 1, a));
    }
  }

  function hud() {
    strings();
    var y = B.hudTop + 12;
    R.text(cache.scoreS, 16, y, 22, '#ffffff', 'left', 'bold', ['rgba(0,0,0,0.6)', 2]);
    R.text(cache.bestS, 16, y + 22, 12, '#b3c5ff', 'left');
    R.text(cache.waveS, W / 2, y, 15, '#ffe082', 'center', 'bold');
    for (var i = 0; i < S.lives; i++) A.ship(W / 2 - 16 + i * 16, y + 24, 0.32, 1);
    UI.pause = { x: W - 52, y: y - 18, w: 40, h: 36 };
    R.circle(W - 32, y, 15, 'rgba(255,255,255,0.18)');
    R.rect(W - 37, y - 7, 3.5, 14, '#ffffff'); R.rect(W - 30.5, y - 7, 3.5, 14, '#ffffff');
    if (S.boss.on) {
      var bw = W - 48, f = Math.max(0, S.boss.hp / S.boss.max);
      R.rrect(24, y + 42, bw, 8, 4, 'rgba(255,255,255,0.2)');
      R.rrect(24, y + 42, Math.max(8, bw * f), 8, 4, f > 0.5 ? '#ff7043' : '#ff1744');
      R.text('首领', 24, y + 58, 11, '#ffab91', 'left');
    }
    // 左下角炸弹键
    var bx = 44, by = H - bottom - 48;
    UI.bomb = { x: bx - 34, y: by - 34, w: 68, h: 68 };
    R.circle(bx, by, 26, S.bombs ? 'rgba(239,83,80,0.85)' : 'rgba(120,120,120,0.5)');
    R.strokeCircle(bx, by, 26, 2, 'rgba(255,255,255,0.7)');
    R.text('炸弹', bx, by - 4, 13, '#ffffff', 'center', 'bold');
    R.text(BOMB_S[S.bombs], bx, by + 12, 11, '#ffffff', 'center');
    // 右下角：火力 / 护盾剩余
    var ry = H - bottom - 30;
    if (S.power > 1) { R.text(POWER_S[S.power], W - 16, ry - 18, 12, '#ffe082', 'right', 'bold'); R.rect(W - 86, ry - 8, 70 * Math.max(0, S.powerT / 14), 3, '#ffca28'); }
    if (S.shieldT > 0) { R.text('护盾', W - 16, ry + 4, 12, '#80deea', 'right', 'bold'); R.rect(W - 86, ry + 14, 70 * S.shieldT / 8, 3, '#26c6da'); }
    if (S.bannerT > 0 && S.state === 'play') {
      R.setAlpha(Math.min(1, S.bannerT * 2));
      R.text(S.banner, W / 2, H * 0.36, 28, S.bossPending ? '#ff5252' : '#ffffff', 'center', 'bold', ['rgba(0,0,0,0.55)', 2]);
      R.setAlpha(1);
    }
  }

  function panel(h) { R.rrect(36, H / 2 - h / 2, W - 72, h, 18, 'rgba(10,14,40,0.88)'); }
  function panels(P) {
    UI.a = UI.b = UI.c = null;
    var cy = H / 2;
    if (S.state === 'ready') {
      panel(300);
      R.text('星际战机', W / 2, cy - 100, 34, '#ffffff', 'center', 'bold', ['#1a237e', 3]);
      R.text('按住屏幕拖动战机，自动开火', W / 2, cy - 50, 14, '#c5cae9', 'center');
      R.text('击落敌机掉落：双 火力  盾 护盾  炸 炸弹', W / 2, cy - 26, 13, '#c5cae9', 'center');
      R.text('每 5 波首领来袭，波次无尽', W / 2, cy - 2, 13, '#c5cae9', 'center');
      R.text(cache.bestS, W / 2, cy + 34, 16, '#ffe082', 'center', 'bold');
      UI.a = R.button('开始游戏', W / 2, cy + 90, 180, 50, '#448aff', '#ffffff');
    } else if (S.state === 'over') {
      panel(320);
      R.text(S.newBest ? '新纪录！' : '战机坠毁', W / 2, cy - 110, 30, S.newBest ? '#ffd740' : '#ffffff', 'center', 'bold');
      R.text(String(S.score), W / 2, cy - 60, 40, '#ffffff', 'center', 'bold');
      R.text('到达第 ' + S.wave + ' 波 · 击落 ' + S.kills + ' 架', W / 2, cy - 18, 14, '#c5cae9', 'center');
      R.text(cache.bestS, W / 2, cy + 10, 15, '#ffe082', 'center');
      if (S.clock - S.overAt > 0.6) UI.a = R.button('再来一局', W / 2, cy + 70, 180, 50, '#448aff', '#ffffff');
      if (S.clock - S.overAt > 0.6) UI.b = R.button('返回标题', W / 2, cy + 128, 140, 40, 'rgba(255,255,255,0.15)', '#ffffff');
    } else if (P.paused) {
      panel(300);
      R.text(P.countdown > 0 ? '准备继续' : '已暂停', W / 2, cy - 100, 28, '#ffffff', 'center', 'bold');
      if (P.countdown > 0) R.text(String(Math.ceil(P.countdown)), W / 2, cy - 50, 40, '#ffe082', 'center', 'bold');
      else R.text('得分 ' + S.score + ' · 第 ' + S.wave + ' 波', W / 2, cy - 52, 15, '#c5cae9', 'center');
      UI.a = R.button('继续', W / 2, cy + 6, 180, 48, '#448aff', '#ffffff');
      UI.b = R.button(P.muted ? '声音：关' : '声音：开', W / 2, cy + 64, 180, 42, 'rgba(255,255,255,0.15)', '#ffffff');
      UI.c = R.button('结束本局', W / 2, cy + 116, 180, 42, 'rgba(255,82,82,0.25)', '#ffcdd2');
    }
  }

  function draw(P) {
    R.begin('#0a0f2e');
    var sh = S.shake;
    R.save();
    if (sh > 0) R.translate((Math.random() - 0.5) * sh, (Math.random() - 0.5) * sh);
    A.background(S.scroll);
    world();
    R.restore();
    if (S.flashT > 0) R.rect(0, 0, W, H, tint(TINT, 1, 1, 0.9, Math.min(0.8, S.flashT * 1.6)));
    if (S.state === 'play') hud();
    else strings();
    panels(P);
    return R.end();
  }
  return { draw: draw };
}

module.exports = { make: make, UI: UI };
