// 画面：牌（背面 / 正面 / 翻面中）、对局 HUD、选难度与主题、结算。
// 翻面 = 横向缩放 |cos(f·π)|，过半换面，同时整体略微放大（像牌被拿起来），模拟绕竖轴的 3D 翻转。
// 不动的牌（朝上 / 朝下停稳、在原位、没缩放）录成一个图层，签名 = 这些牌的 id 与朝向的哈希；只有翻面 / 发牌 / 弹跳中的牌每帧现画。
var Themes = require('themes.js');
var LEVELS = require('round.js').LEVELS;

function make(R, W, H, top) {
  var btns = [];
  function btn(label, cx, cy, w, h, bg, fg, fn, arg) {
    var b = R.button(label, cx, cy, w, h, bg, fg); b.fn = fn; b.arg = arg; btns.push(b); return b;
  }
  function hitbox(x, y, w, h, fn, arg) { btns.push({ x: x, y: y, w: w, h: h, fn: fn, arg: arg }); }

  function background() {
    R.layer('bg', W + 'x' + H, function () {
      R.gradRect(0, 0, W, H, [[0, '#e0f7fa'], [1, '#ede7f6']], true);
      [[40, 140, 60], [340, 260, 46], [60, 620, 54], [330, 560, 70]].forEach(function (c) { R.circle(c[0], c[1], c[2], 'rgba(255,255,255,0.4)'); });
    });
  }

  // ───────── 牌 ─────────
  function back(theme, w, h) {
    var r = Math.min(w, h) * 0.14;
    R.rrect(-w / 2, -h / 2 + 2, w, h, r, 'rgba(0,0,0,0.12)');
    R.rrect(-w / 2, -h / 2, w, h, r, theme.back[1]);
    R.rrect(-w / 2 + 4, -h / 2 + 4, w - 8, h - 8, r * 0.7, theme.back[0]);
    var d = Math.min(w, h) * 0.22;
    R.poly([[0, -d], [d, 0], [0, d], [-d, 0]], 'rgba(255,255,255,0.25)');
    R.text('?', 0, 1, Math.round(Math.min(w, h) * 0.3), 'rgba(255,255,255,0.9)', 'center', 'bold');
  }
  function face(theme, c) {
    var w = c.w, h = c.h, r = Math.min(w, h) * 0.14;
    R.rrect(-w / 2, -h / 2 + 2, w, h, r, 'rgba(0,0,0,0.12)');
    R.rrect(-w / 2, -h / 2, w, h, r, c.matched ? '#66bb6a' : '#ffffff');
    if (c.matched) R.rrect(-w / 2 + 3, -h / 2 + 3, w - 6, h - 6, r * 0.8, '#ffffff');
    theme.draw(R, c.item, 0, 0, w, h);
  }
  function card(theme, c) {
    var k = Math.cos(c.f * Math.PI), sx = Math.abs(k), lift = Math.sin(c.f * Math.PI) * 0.08;
    if (sx < 0.02) return;
    R.save();
    R.translate(c.x + c.w / 2, c.y + c.h / 2);
    R.scale(sx * (1 + lift) * c.s, (1 + lift) * c.s);
    if (c.f > 0.5) face(theme, c); else back(theme, c.w, c.h);
    R.restore();
  }
  function isStatic(c) {
    return (c.f < 0.001 || c.f > 0.999) && Math.abs(c.s - 1) < 0.001 && Math.abs(c.x - c.hx) < 0.01 && Math.abs(c.y - c.hy) < 0.01;
  }
  var rnd = null;
  function recordStatic() {
    for (var i = 0; i < rnd.cards.length; i++) { var c = rnd.cards[i]; if (isStatic(c)) card(rnd.theme, c); }
  }
  function cards(round) {
    var h = 17;
    for (var i = 0; i < round.cards.length; i++) {
      var c = round.cards[i];
      h = (Math.imul(h, 31) + (isStatic(c) ? c.id * 4 + (c.f > 0.5 ? 1 : 0) + (c.matched ? 2 : 0) : 0)) | 0;
    }
    rnd = round;
    R.layer('cards', h + ':' + round.theme.id, recordStatic);
    for (var j = 0; j < round.cards.length; j++) if (!isStatic(round.cards[j])) card(round.theme, round.cards[j]);
  }

  function clock(t) { var s = Math.floor(t); return Math.floor(s / 60) + ':' + (s % 60 < 10 ? '0' : '') + (s % 60); }

  // ───────── 对局 HUD ─────────
  function hud(S, round, A) {
    var y = top, x = 12, w = W - 24;
    R.rrect(x, y, w, 60, 14, 'rgba(255,255,255,0.85)');
    var cols = [['用时', clock(round.time)], ['步数', String(round.steps)], ['得分', String(round.score)],
      ['连对', round.combo > 1 ? '×' + round.combo : '—']];
    cols.forEach(function (c, i) {
      var cx = x + w * (i + 0.5) / 4;
      R.text(c[0], cx, y + 18, 12, '#7e57c2', 'center');
      R.text(c[1], cx, y + 40, 20, i === 3 && round.combo > 1 ? '#ef6c00' : '#37474f', 'center', 'bold');
    });
    R.text(round.level.name + ' · ' + round.theme.name + ' · ' + round.matchedPairs + '/' + round.pairs + ' 对', W / 2, y + 74, 13, '#5e35b1', 'center');
    var by = S.btnY;
    btn('菜单', W / 2 - 112, by, 96, 38, '#ffffff', '#5e35b1', A.menu);
    btn('重来', W / 2, by, 96, 38, '#ffffff', '#5e35b1', A.retry);
    btn(S.music ? '音乐：开' : '音乐：关', W / 2 + 112, by, 96, 38, '#ffffff', '#5e35b1', A.music);
  }

  // ───────── 选难度 / 主题 ─────────
  function menu(S, A) {
    R.text('记忆翻牌', W / 2, top + 30, 34, '#5e35b1', 'center', 'bold', ['#ffffff', 3]);
    R.text('翻开两张一样的就配上了，连着配上分数更高', W / 2, top + 66, 13, '#7e57c2', 'center');
    // 难度
    LEVELS.forEach(function (lv, i) {
      var y = top + 96 + i * 82, x = 24, w = W - 48, h = 70, best = S.best[lv.id];
      R.rrect(x, y + 3, w, h, 16, 'rgba(94,53,177,0.25)');
      R.rrect(x, y, w, h, 16, '#ffffff');
      R.rrect(x + 10, y + 10, 50, 50, 12, ['#26c6da', '#7e57c2', '#ec407a'][i]);
      R.text(lv.cols + '×' + lv.rows, x + 35, y + 35, 15, '#ffffff', 'center', 'bold');
      R.text(lv.name + '  ' + lv.cols * lv.rows / 2 + ' 对', x + 74, y + 24, 18, '#37474f', 'left', 'bold');
      R.text(best ? '最佳 ' + best.score + ' 分 · ' + best.steps + ' 步 · ' + clock(best.time) : '还没有成绩', x + 74, y + 50, 13, '#78909c', 'left');
      R.text('开始 ›', x + w - 16, y + 35, 15, '#5e35b1', 'right', 'bold');
      hitbox(x, y, w, h, A.play, lv.id);
    });
    // 卡面主题
    var ty = top + 96 + 3 * 82 + 18;
    R.text('卡面主题', 24, ty, 15, '#37474f', 'left', 'bold');
    var ld = Themes.loading();
    Themes.META.forEach(function (m, i) {
      var cw = (W - 48 - 20) / 3, cx = 24 + i * (cw + 10), cy = ty + 18, sel = S.theme === m.id;
      R.rrect(cx, cy, cw, 54, 12, sel ? '#5e35b1' : '#ffffff');
      R.text(m.name, cx + cw / 2, cy + 20, 16, sel ? '#ffffff' : '#5e35b1', 'center', 'bold');
      var sub = !m.pkg ? '主包' : ld && ld.id === m.id ? '加载中 ' + Math.round(ld.progress) + '%' : Themes.loaded(m.id) ? '分包 · 已加载' : '分包 · 点选加载';
      R.text(sub, cx + cw / 2, cy + 40, 11, sel ? '#d1c4e9' : '#9575cd', 'center');
      hitbox(cx, cy, cw, 54, A.theme, m.id);
    });
    btn(S.music ? '背景音乐：开' : '背景音乐：关', W / 2, ty + 110, 180, 42, '#ffffff', '#5e35b1', A.music);
    if (S.toast) R.text(S.toast, W / 2, ty + 150, 13, '#e53935', 'center');
  }

  // ───────── 结算 ─────────
  function result(S, round, A) {
    var res = round.result, cx = W / 2, cy = H / 2, pw = 300, ph = 360;
    R.rect(0, 0, W, H, 'rgba(0,0,0,0.4)');
    R.rrect(cx - pw / 2, cy - ph / 2, pw, ph, 20, '#ffffff');
    R.text('全部配对！', cx, cy - ph / 2 + 38, 26, '#5e35b1', 'center', 'bold');
    for (var s = 0; s < 3; s++) {
      var on = s < res.stars, k = on ? Math.min(1, Math.max(0, (S.resultAge - s * 0.2) * 4)) : 1;
      star(cx - 64 + s * 64, cy - ph / 2 + 90 - (s === 1 ? 8 : 0), 22 * (on ? 0.6 + 0.4 * k : 1), on && k > 0);
    }
    R.text('得分 ' + res.score, cx, cy - 18, 24, '#37474f', 'center', 'bold');
    R.text('用时 ' + clock(res.time) + '   步数 ' + res.steps + '   最多连对 ' + res.maxCombo, cx, cy + 14, 13, '#78909c', 'center');
    R.text(res.bonus ? '速度奖励 +' + res.bonus : '再快一点有速度奖励', cx, cy + 36, 13, '#78909c', 'center');
    if (S.newBest) R.text('新纪录！', cx, cy + 60, 15, '#e53935', 'center', 'bold');
    btn('再来一局', cx, cy + 100, 200, 46, '#5e35b1', '#ffffff', A.retry);
    btn('换难度 / 主题', cx, cy + 146, 160, 34, '#ede7f6', '#5e35b1', A.menu);
  }
  function star(cx, cy, r, on) {
    var p = [];
    for (var i = 0; i < 10; i++) { var a = -Math.PI / 2 + Math.PI * i / 5, rr = i % 2 ? r * 0.45 : r; p.push([cx + Math.cos(a) * rr, cy + Math.sin(a) * rr]); }
    for (var k = 0; k < 10; k++) R.poly([[cx, cy], p[k], p[(k + 1) % 10]], on ? '#ffc107' : '#eceff1');
  }

  return { btns: btns, background: background, cards: cards, hud: hud, menu: menu, result: result, clock: clock,
    reset: function () { btns.length = 0; } };
}
module.exports = { make: make };
