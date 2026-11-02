// 画面：背景、关卡选择、对局 HUD（步数 / 目标 / 分数条与星线）、结算面板。
// 可点的东西用 btn() 登记命中框（每帧重建），game.js 的 tap 按登记倒序（上层先）判。
var F = require('fruit.js');
var LV = require('levels.js');

function make(R, W, H, top) {
  var btns = [];
  function btn(label, cx, cy, w, h, bg, fg, fn) {
    var b = R.button(label, cx, cy, w, h, bg, fg); b.fn = fn; btns.push(b); return b;
  }
  function fruitIcon(t, x, y, r, s) { R.sprite('fi' + t + '_' + r, 1, function () { F.fruit(R, t, r); }, x, y, s || 1, 1); }
  function starShape(cx, cy, r, on) {
    F.star(R, cx, cy, r, r * 0.45, 5, -Math.PI / 2, on ? '#ffc107' : 'rgba(0,0,0,0.15)');
  }

  function background() {
    R.layer('bg', W + 'x' + H, function () {
      R.gradRect(0, 0, W, H, [[0, '#ffd6e7'], [0.55, '#fff1d0'], [1, '#d7f3d0']], true);
      // 几朵淡淡的云 / 光斑
      [[60, 120, 50], [320, 90, 38], [300, 600, 60], [40, 560, 44]].forEach(function (c) { R.circle(c[0], c[1], c[2], 'rgba(255,255,255,0.35)'); });
    });
  }

  // ───────── 关卡选择 ─────────
  function menu(S, A) {
    R.text('萌果消消乐', W / 2, top + 30, 34, '#ff7043', 'center', 'bold', ['#ffffff', 3]);
    for (var i = 0; i < 6; i++) fruitIcon(i, W / 2 - 125 + i * 50, top + 82 + Math.sin(S.clock * 3 + i) * 4, 18);
    var stars = 0; for (var k in S.prog.stars) stars += S.prog.stars[k];
    R.text('已得 ' + stars + ' / ' + LV.list.length * 3 + ' 星    总分 ' + S.total(), W / 2, top + 122, 15, '#795548', 'center');
    var cols = 4, bw = 70, gap = 12, gx = (W - cols * bw - (cols - 1) * gap) / 2, gy = top + 146;
    // 关卡格录成图层（解锁 / 星数变了才重录），命中框每帧登记
    var sig = S.prog.unlocked + ':';
    LV.list.forEach(function (lv, idx) {
      sig += S.prog.stars[lv.no] || 0;
      var c = idx % cols, r = Math.floor(idx / cols), x = gx + c * (bw + gap), y = gy + r * (bw + 24);
      if (lv.no <= S.prog.unlocked) btns.push({ x: x, y: y, w: bw, h: bw, no: lv.no, fn: A.play });
    });
    menuS = S;
    R.layer('levels', sig, levelsFn);
    var by = gy + 3 * (bw + 24) + 20;
    btn('开始第 ' + Math.min(S.prog.unlocked, LV.list.length) + ' 关', W / 2, by, 220, 50, '#43a047', '#ffffff', function () { A.play(Math.min(S.prog.unlocked, LV.list.length)); });
    btn('排行榜', W / 2 - 62, by + 64, 110, 40, '#ffffff', '#e65100', A.rank);
    btn(S.muted ? '音效：关' : '音效：开', W / 2 + 62, by + 64, 110, 40, '#ffffff', '#6d4c41', A.mute);
  }
  var menuS = null;
  function levelsFn() {
    var S = menuS, cols = 4, bw = 70, gap = 12, gx = (W - cols * bw - (cols - 1) * gap) / 2, gy = top + 146;
    LV.list.forEach(function (lv, idx) {
      var c = idx % cols, r = Math.floor(idx / cols), x = gx + c * (bw + gap), y = gy + r * (bw + 24);
      var locked = lv.no > S.prog.unlocked, st = S.prog.stars[lv.no] || 0;
      R.rrect(x, y + 3, bw, bw, 16, locked ? 'rgba(0,0,0,0.12)' : '#e65100');
      R.rrect(x, y, bw, bw, 16, locked ? '#d7ccc8' : '#ff9800');
      if (locked) {
        R.rrect(x + bw / 2 - 12, y + bw / 2 - 4, 24, 18, 4, '#8d6e63');
        R.strokeCircle(x + bw / 2, y + bw / 2 - 6, 8, 3, '#8d6e63');
      } else {
        R.text(String(lv.no), x + bw / 2, y + bw / 2 - 6, 26, '#ffffff', 'center', 'bold');
        for (var s = 0; s < 3; s++) starShape(x + bw / 2 - 16 + s * 16, y + bw - 14, 7, s < st);
      }
    });
  }

  // ───────── 对局 HUD ─────────
  // HUD 只在步数 / 分数 / 目标变了时重录（录成图层，平时一帧一次 drawArrays）；底下三个按钮录一次，命中框每帧登记
  var hudM = null;
  function hudFn() { hudBody(hudM); }
  function btnsFn() {
    var bottom = top + 116 + cellSize * 8 + 34;
    R.button('关卡', W / 2 - 112, bottom, 96, 38, '#ffffff', '#6d4c41');
    R.button('重来', W / 2, bottom, 96, 38, '#ffffff', '#6d4c41');
    R.button('排行', W / 2 + 112, bottom, 96, 38, '#ffffff', '#e65100');
  }
  var cellSize = 0;
  function hud(S, M, A) {
    hudM = M; cellSize = S.geo.CELL;
    var sig = M.no + '|' + M.moves + '|' + M.score;
    for (var i = 0; i < M.goals.length; i++) sig += '|' + M.goals[i].left;
    R.layer('hud', sig, hudFn);
    R.layer('pbtn', cellSize, btnsFn);
    var bottom = S.geo.OY + S.geo.CELL * 8 + 34;
    btns.push({ x: W / 2 - 112 - 48, y: bottom - 19, w: 96, h: 38, fn: A.menu });
    btns.push({ x: W / 2 - 48, y: bottom - 19, w: 96, h: 38, fn: A.retry });
    btns.push({ x: W / 2 + 112 - 48, y: bottom - 19, w: 96, h: 38, fn: A.rank });
  }
  function hudBody(M) {
    var lv = M.lv, x = 12, y = top, w = W - 24, h = 104;
    R.rrect(x, y, w, h, 16, 'rgba(255,255,255,0.88)');
    R.text('第 ' + lv.no + ' 关', x + 46, y + 22, 16, '#6d4c41', 'center', 'bold');
    R.rrect(x + 10, y + 36, 72, 56, 12, M.moves <= 5 ? '#ef5350' : '#ff9800');
    R.text(String(M.moves), x + 46, y + 58, 26, '#ffffff', 'center', 'bold');
    R.text('步', x + 46, y + 82, 11, '#fff3e0', 'center');
    // 目标：小水果 + 剩余数（收齐打勾）
    var gx = x + 100, gw = (w - 110) / Math.max(3, M.goals.length);
    M.goals.forEach(function (g, i) {
      var cx = gx + gw * i + gw / 2;
      R.save(); R.translate(cx, y + 32); F.fruit(R, g.t, 14); R.restore();   // 图层里不能套精灵：直接画
      if (g.left > 0) R.text(String(g.left), cx, y + 58, 15, '#5d4037', 'center', 'bold');
      else R.text('✓', cx, y + 58, 18, '#43a047', 'center', 'bold');
    });
    // 分数条：两条星线（star2 / star3），满条 = star3 × 1.15
    var bx = gx, bw = w - 110, byy = y + 78, full = lv.star3 * 1.15, k = Math.min(1, M.score / full);
    R.rrect(bx, byy, bw, 12, 6, '#efebe9');
    if (k > 0) R.rrect(bx, byy, Math.max(12, bw * k), 12, 6, '#ffb300');
    [lv.star2, lv.star3].forEach(function (s, i) { starShape(bx + bw * s / full, byy + 6, 8, M.score >= s); });
    R.text(String(M.score), bx + bw, byy - 10, 13, '#e65100', 'right', 'bold');
  }

  // 棋盘底：格子棋盘录成图层
  function boardBg(g) {
    R.layer('board', g.CELL, function () {
      R.rrect(g.OX - 6, g.OY - 6, g.CELL * 8 + 12, g.CELL * 8 + 12, 14, 'rgba(121,85,72,0.25)');
      for (var r = 0; r < 8; r++) for (var c = 0; c < 8; c++)
        R.rrect(g.OX + c * g.CELL + 1, g.OY + r * g.CELL + 1, g.CELL - 2, g.CELL - 2, 8, (r + c) % 2 ? 'rgba(255,255,255,0.55)' : 'rgba(255,255,255,0.75)');
    });
  }

  // ───────── 结算 ─────────
  function result(S, M, A) {
    var res = M.result, cx = W / 2, cy = H / 2, pw = 300, ph = 350;
    R.rect(0, 0, W, H, 'rgba(0,0,0,0.45)');
    R.rrect(cx - pw / 2, cy - ph / 2, pw, ph, 20, '#fffaf0');
    R.text(res.win ? '过关啦！' : '步数用完了', cx, cy - ph / 2 + 40, 28, res.win ? '#ff7043' : '#8d6e63', 'center', 'bold');
    for (var s = 0; s < 3; s++) {
      var on = s < res.stars, sx = cx - 70 + s * 70, sy = cy - ph / 2 + 100 - (s === 1 ? 10 : 0);
      var pop = on ? Math.min(1, Math.max(0, (S.resultAge - s * 0.25) * 4)) : 1;
      starShape(sx, sy, 26 * (on ? 0.6 + 0.4 * pop : 1), on && pop > 0);
    }
    R.text('得分 ' + res.score, cx, cy - 10, 22, '#5d4037', 'center', 'bold');
    R.text(res.win ? (res.bonus ? '剩余步数奖励 +' + res.bonus : '步数刚好用完') : '目标还差 ' + M.goals.map(function (g) {
      return g.left > 0 ? LV.NAMES[g.t] + g.left : '';
    }).filter(Boolean).join('、'), cx, cy + 22, 14, '#8d6e63', 'center');
    if (S.newBest) R.text('本关新纪录！', cx, cy + 48, 15, '#e53935', 'center', 'bold');
    var hasNext = res.win && M.no < LV.list.length;
    btn(hasNext ? '下一关' : '再玩一次', cx, cy + 92, 200, 46, '#43a047', '#ffffff', hasNext ? A.next : A.retry);
    btn('关卡', cx - 52, cy + 142, 92, 36, '#ffffff', '#6d4c41', A.menu);
    btn('排行', cx + 52, cy + 142, 92, 36, '#ffffff', '#e65100', A.rank);
  }

  return {
    btns: btns, background: background, menu: menu, hud: hud, boardBg: boardBg, result: result, btn: btn, fruitIcon: fruitIcon,
    reset: function () { btns.length = 0; }
  };
}
module.exports = { make: make };
