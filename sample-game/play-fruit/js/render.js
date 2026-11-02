// 绘制：木板（图层，录一次）→ 果渍 → 菜单水果 / 飞行的水果与炸弹 / 半个 → 果汁粒子 → 刀光 → 飘字 → HUD / 面板。
// 按钮命中框写进 UI，game.js 用它判点按。
var B = require('./base.js');
var Wd = require('./world.js');
var F = require('./fruits.js');
var Blade = require('./blade.js');
var S = Wd.S, W = B.W, H = B.H, TAU = Math.PI * 2;
var UI = { pause: null, a: null, b: null, c: null };
var TINT = [1, 1, 1, 1];
var FLASH_C = [1, 1, 1, 1];

var cache = { score: -1, scoreS: '', sec: -1, secS: '', best: -1, bestS: '' };
function strings() {
  if (cache.score !== S.score) { cache.score = S.score; cache.scoreS = String(S.score); }
  var b = S.best[S.mode];
  if (cache.best !== b) { cache.best = b; cache.bestS = '最高 ' + b; }
  var sec = Math.ceil(S.timeLeft);
  if (cache.sec !== sec) { cache.sec = sec; cache.secS = Math.floor(sec / 60) + ':' + (sec % 60 < 10 ? '0' : '') + (sec % 60); }
}

function make(R) {
  var Fr = F.make(R);
  function disc(c, x, y, r, color) { c.beginPath(); c.arc(x, y, r, 0, TAU); c.fillStyle = color; c.fill(); }
  var glow = R.bake(32, 32, function (c, w, h) {
    for (var i = 8; i >= 1; i--) disc(c, w / 2, h / 2, w / 2 * i / 8, 'rgba(255,255,255,' + (0.06 + (8 - i) * 0.018) + ')');
    disc(c, w / 2, h / 2, w * 0.12, '#ffffff');
  });
  var drop = R.bake(24, 24, function (c, w, h) { disc(c, w / 2, h / 2, w / 2, 'rgba(255,255,255,0.45)'); disc(c, w / 2, h / 2, w * 0.36, '#ffffff'); });
  // 果渍：几团圆叠成不规则的一摊（固定种子，每次一样）
  var splat = R.bake(64, 64, function (c, w, h) {
    var s = 7;
    function rnd() { s = (s * 16807) % 2147483647; return s / 2147483647; }
    disc(c, w / 2, h / 2, w * 0.27, '#ffffff');
    for (var i = 0; i < 9; i++) {
      var a = i / 9 * TAU + rnd() * 0.5, d = w * (0.18 + rnd() * 0.16);
      disc(c, w / 2 + Math.cos(a) * d, h / 2 + Math.sin(a) * d, w * (0.05 + rnd() * 0.08), '#ffffff');
    }
  });
  var bottom = Math.max(H - (B.safe.bottom || H), 0);

  function board() {
    var pw = W / 5;
    // 不给木板分深浅：两块半透明整列盖上去是 40% 屏的混合填充，无头 CPU 后端上 ~1.5ms；深浅靠缝和木纹就够看
    for (var i = 1; i < 5; i++) { R.rect(i * pw - 1.5, 0, 3, H, '#4e2c17'); R.rect(i * pw + 1.5, 0, 1, H, 'rgba(255,220,180,0.12)'); }
    var s = 11;
    function rnd() { s = (s * 16807) % 2147483647; return s / 2147483647; }
    for (var g = 0; g < 26; g++) {                     // 木纹：细长的浅 / 深条
      var x = rnd() * W, y = rnd() * H, l = 60 + rnd() * 160;
      R.rect(x, y, 1, l, g % 2 ? 'rgba(60,30,10,0.18)' : 'rgba(255,220,180,0.08)');
    }
    for (var k = 0; k < 4; k++) R.ellipse(rnd() * W, rnd() * H, 6 + rnd() * 5, 12 + rnd() * 8, 0, 'rgba(60,30,10,0.22)');
  }

  function fruitAt(k, x, y, a, isHalf) {
    R.save(); R.translate(x, y); R.rotate(a);
    if (k < 0) Fr.bomb(S.clock, glow); else if (isHalf) Fr.half(k); else Fr.whole(k);
    R.restore();
  }
  function world() {
    var i;
    for (i = 0; i < Wd.splats.length; i++) {
      var s = Wd.splats[i]; if (!s.on) continue;
      s.c[3] = 0.55 * Math.min(1, (3 - s.t) / 1.2);
      R.stamp(splat, s.x, s.y, s.s, s.c);
    }
    if (S.state === 'ready') for (i = 0; i < S.menu.length; i++) {
      var m = S.menu[i], r = F.TYPES[m.k].r;
      R.strokeCircle(m.x, m.y, r + 22, 3, 'rgba(255,255,255,0.35)');
      if (m.on) fruitAt(m.k, m.x, m.y + Math.sin(S.clock * 2 + i) * 4, m.a, false);
    }
    for (i = 0; i < Wd.fruits.length; i++) { var f = Wd.fruits[i]; if (f.on) fruitAt(f.k, f.x, f.y, f.a, false); }
    for (i = 0; i < Wd.halves.length; i++) { var h = Wd.halves[i]; if (h.on) fruitAt(h.k, h.x, h.y, h.a, true); }
    for (i = 0; i < Wd.drops.length; i++) {
      var p = Wd.drops[i]; if (!p.on) continue;
      var k = p.life / p.max;
      TINT[0] = p.r; TINT[1] = p.g; TINT[2] = p.b; TINT[3] = Math.min(1, k * 1.5);
      R.stamp(drop, p.x, p.y, p.s * (0.5 + 0.5 * k), TINT);
    }
    Blade.draw(R, S.clock);
    for (i = 0; i < Wd.pops.length; i++) {
      var q = Wd.pops[i]; if (!q.on) continue;
      R.setAlpha(Math.min(1, (1 - q.t) * 3));
      R.text(q.s, q.x, q.y, q.size, q.c, 'center', 'bold', ['rgba(40,20,0,0.7)', 2]);
      R.setAlpha(1);
    }
  }

  function hud() {
    strings();
    var y = B.hudTop + 14;
    R.save(); R.translate(28, y + 2); R.scale(0.55, 0.55); R.rotate(Math.PI); Fr.half(0); R.restore();
    R.text(cache.scoreS, 52, y, 28, '#ffeb3b', 'left', 'bold', ['#5d2e0c', 2.5]);
    R.text(cache.bestS, 16, y + 30, 13, '#ffe0b2', 'left', 'bold', ['rgba(60,30,10,0.6)', 1.5]);
    UI.pause = { x: W - 52, y: y - 18, w: 40, h: 36 };
    R.circle(W - 32, y, 15, 'rgba(0,0,0,0.25)');
    R.rect(W - 37, y - 7, 3.5, 14, '#ffffff'); R.rect(W - 30.5, y - 7, 3.5, 14, '#ffffff');
    if (S.mode === 'classic') {
      for (var i = 0; i < 3; i++) {
        var lost = i >= S.lives;
        R.text('×', W - 132 + i * 30, y, lost ? 34 : 26, lost ? '#e53935' : 'rgba(255,255,255,0.45)', 'center', 'bold', lost ? ['#ffffff', 1.5] : null);
      }
    } else {
      var warn = S.timeLeft <= 10;
      R.text(cache.secS, W / 2, y, 26, warn ? '#ff5252' : '#ffffff', 'center', 'bold', ['rgba(60,30,10,0.7)', 2]);
    }
  }

  function panel(h) { R.rrect(36, H / 2 - h / 2, W - 72, h, 18, 'rgba(40,20,8,0.86)'); }
  function panels(P) {
    UI.a = UI.b = UI.c = null;
    var cy = H / 2;
    if (S.state === 'ready') {
      R.text('切水果', W / 2, H * 0.24, 46, '#ffeb3b', 'center', 'bold', ['#bf360c', 3]);
      R.text('手指划过水果把它切开，别碰炸弹', W / 2, H * 0.24 + 48, 15, '#fff3e0', 'center', 'bold', ['rgba(60,30,10,0.7)', 1.5]);
      R.text('划开下面的水果选择模式', W / 2, H * 0.6 - 92, 14, '#ffe0b2', 'center', 'normal', ['rgba(60,30,10,0.7)', 1.5]);
      for (var i = 0; i < S.menu.length; i++) {
        var m = S.menu[i];
        R.text(m.mode === 'classic' ? '经典模式' : '限时 60 秒', m.x, m.y + 66, 17, '#ffffff', 'center', 'bold', ['rgba(60,30,10,0.8)', 2]);
        R.text('最高 ' + S.best[m.mode], m.x, m.y + 90, 13, '#ffe082', 'center', 'bold', ['rgba(60,30,10,0.7)', 1.5]);
      }
      R.text('经典：漏 3 个或切到炸弹结束', W / 2, H * 0.6 + 140, 13, '#fff3e0', 'center', 'normal', ['rgba(60,30,10,0.7)', 1.5]);
      R.text('限时：切到炸弹扣 10 分 · 连切 3 个以上有奖励', W / 2, H * 0.6 + 162, 13, '#fff3e0', 'center', 'normal', ['rgba(60,30,10,0.7)', 1.5]);
    } else if (S.state === 'over') {
      panel(330);
      var title = S.newBest ? '新纪录！' : S.mode === 'timed' ? '时间到' : S.lives <= 0 ? '漏掉太多了' : '碰到炸弹了';
      R.text(title, W / 2, cy - 115, 30, S.newBest ? '#ffd740' : '#ffffff', 'center', 'bold');
      R.text(String(S.score), W / 2, cy - 62, 44, '#ffeb3b', 'center', 'bold');
      R.text((S.mode === 'classic' ? '经典模式' : '限时模式') + ' · 最高 ' + S.best[S.mode], W / 2, cy - 18, 14, '#ffe0b2', 'center');
      R.text('切开 ' + S.cuts + ' 个 · 最大连切 ' + S.maxCombo, W / 2, cy + 8, 14, '#ffe0b2', 'center');
      if (S.clock - S.overAt > 0.6) {
        UI.a = R.button('再来一局', W / 2, cy + 68, 180, 50, '#ff7043', '#ffffff');
        UI.b = R.button('返回菜单', W / 2, cy + 126, 140, 40, 'rgba(255,255,255,0.15)', '#ffffff');
      }
    } else if (P.paused) {
      panel(300);
      R.text(P.countdown > 0 ? '准备继续' : '已暂停', W / 2, cy - 100, 28, '#ffffff', 'center', 'bold');
      if (P.countdown > 0) R.text(String(Math.ceil(P.countdown)), W / 2, cy - 50, 40, '#ffe082', 'center', 'bold');
      else R.text('得分 ' + S.score, W / 2, cy - 52, 15, '#ffe0b2', 'center');
      UI.a = R.button('继续', W / 2, cy + 6, 180, 48, '#ff7043', '#ffffff');
      UI.b = R.button(P.muted ? '声音：关' : '声音：开', W / 2, cy + 64, 180, 42, 'rgba(255,255,255,0.15)', '#ffffff');
      UI.c = R.button('返回菜单', W / 2, cy + 116, 180, 42, 'rgba(255,255,255,0.15)', '#ffffff');
    }
  }

  function draw(P) {
    R.begin('#8a5530');
    R.save();
    if (S.shake > 0) R.translate((Math.random() - 0.5) * S.shake, (Math.random() - 0.5) * S.shake);
    R.layer('board', 1, board);
    world();
    R.restore();
    if (S.flashT > 0) { FLASH_C[3] = Math.min(0.9, S.flashT); R.rect(0, 0, W, H, FLASH_C); }
    if (S.state === 'play' || S.state === 'boom') hud();
    panels(P);
    return R.end();
  }
  return { draw: draw, bottom: bottom };
}

module.exports = { make: make, UI: UI };
