// 数独的画面（WebGL，js/gl2d.js）：版面、棋盘（高亮 / 冲突 / 笔记）、工具栏、数字键盘、菜单、出题进度。
// 每个 draw* 返回命中框，game.js 拿去判点按。网格线不变，录成图层（每帧一次 drawArrays）。
var B = require('./base.js');
var Gen = require('./gen.js');
var W = B.W, H = B.H;

// ───────── 版面（逻辑像素） ─────────
var top = B.hudTop + 46;
var CELL = Math.floor(Math.min(W - 16, H - top - 178) / 9);
var SIZE = CELL * 9, BX = Math.floor((W - SIZE) / 2), BY = top;
var TOOL_CY = BY + SIZE + 38, PAD_CY = TOOL_CY + 66;
var L = { cell: CELL, size: SIZE, x: BX, y: BY, toolCy: TOOL_CY, padCy: PAD_CY };

var C = {
  bg: '#f6f1e7', board: '#fffdf8', peer: '#ece6d8', same: '#d9e6f7', sel: '#b8d0f2', confBg: '#fbe0dc',
  given: '#2d2a26', user: '#2f6fd6', hint: '#2e9e5b', conf: '#d9480f', note: '#7a7468', thin: '#d6cfbf', thick: '#4a4237',
  accent: '#e8590c', btn: '#ffffff', btnText: '#4a4237'
};

function mmss(t) {
  t = Math.floor(t);
  var m = Math.floor(t / 60), s = t % 60;
  return (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
}

// ───────── 顶栏 ─────────
function hud(R, st) {
  var cy = B.hudTop + 18, out = {};
  out.menu = R.button('菜单', 50, cy, 72, 32, '#ffffff', C.btnText);
  out.fresh = R.button('新局', W - 50, cy, 72, 32, '#ffffff', C.btnText);
  R.text(Gen.DIFFS[st.p.diff].name + '  ' + mmss(st.p.time), W / 2, cy, 18, C.given, 'center', 'bold');
  return out;
}

// ───────── 棋盘 ─────────
function board(R, st) {
  var p = st.p, sel = st.sel, sv = sel >= 0 ? p.val[sel] : 0;
  R.rrect(BX - 4, BY - 4, SIZE + 8, SIZE + 8, 10, C.thick);
  R.rect(BX, BY, SIZE, SIZE, C.board);
  var sr = sel >= 0 ? (sel / 9) | 0 : -1, sc = sel % 9, sb = sel >= 0 ? ((sr / 3) | 0) * 3 + ((sc / 3) | 0) : -1;
  for (var i = 0; i < 81; i++) {
    var r = (i / 9) | 0, c = i % 9, x = BX + c * CELL, y = BY + r * CELL, bg = null;
    if (st.wave >= 0) {                              // 完成时的对角线波浪
      var k = st.wave * 18 - (r + c);
      if (k > 0 && k < 3) bg = 'rgba(46,158,91,' + (0.55 - Math.abs(k - 1.5) * 0.3).toFixed(2) + ')';
    } else if (i === sel) bg = C.sel;
    else if (p.conf[i]) bg = C.confBg;
    else if (sv && p.val[i] === sv) bg = C.same;
    else if (sel >= 0 && (r === sr || c === sc || (((r / 3) | 0) * 3 + ((c / 3) | 0)) === sb)) bg = C.peer;
    if (bg) R.rect(x, y, CELL, CELL, bg);
    var v = p.val[i];
    if (v) {
      var col = p.conf[i] && !p.given[i] ? C.conf : p.given[i] ? C.given : p.hinted[i] ? C.hint : C.user;
      R.text(String(v), x + CELL / 2, y + CELL / 2 + 1, Math.round(CELL * 0.6), col, 'center', p.given[i] ? 'bold' : 'normal');
      if (p.conf[i] && p.given[i]) R.rect(x + CELL * 0.3, y + CELL - 5, CELL * 0.4, 2, C.conf);   // 题目数字被冲突时下面画一道
    } else if (p.notes[i]) {
      var m = p.notes[i], ns = Math.max(9, Math.round(CELL * 0.3)), q = CELL / 3;
      for (var d = 1; d <= 9; d++) {
        if (!(m & (1 << (d - 1)))) continue;
        var nx = x + ((d - 1) % 3) * q + q / 2, ny = y + (((d - 1) / 3) | 0) * q + q / 2 + 0.5;
        if (d === sv) R.circle(nx, ny, q * 0.42, '#c5d8f5');
        R.text(String(d), nx, ny, ns, C.note, 'center');
      }
    }
  }
  // 网格线不随局面变：录成图层
  R.layer('grid', CELL, function () {
    for (var k = 1; k < 9; k++) {
      var thick = k % 3 === 0, w = thick ? 2 : 1, col = thick ? C.thick : C.thin;
      R.rect(BX + k * CELL - w / 2, BY, w, SIZE, col);
      R.rect(BX, BY + k * CELL - w / 2, SIZE, w, col);
    }
  });
  return { x: BX, y: BY, w: SIZE, h: SIZE };
}

// ───────── 工具栏：撤销 / 擦除 / 笔记 / 提示 ─────────
function tools(R, st) {
  var out = {}, n = 4, w = (SIZE - 3 * 10) / 4, h = 50;
  var items = [
    ['undo', '撤销', ''],
    ['erase', '擦除', ''],
    ['notes', '笔记', st.notes ? '开' : '关'],
    ['hint', '提示', String(st.p.hints)]
  ];
  for (var k = 0; k < n; k++) {
    var it = items[k], cx = BX + w / 2 + k * (w + 10), on = it[0] === 'notes' && st.notes;
    R.rrect(cx - w / 2, TOOL_CY - h / 2, w, h, 12, on ? C.accent : C.btn);
    R.text(it[1], cx, TOOL_CY - (it[2] ? 7 : 0), 15, on ? '#ffffff' : C.btnText, 'center', 'bold');
    if (it[2]) R.text(it[2], cx, TOOL_CY + 13, 12, on ? '#ffe8d6' : (it[0] === 'hint' && !st.p.hints ? '#b0a898' : C.accent), 'center', 'bold');
    if (it[0] === 'undo' && !st.p.undo.length) R.rrect(cx - w / 2, TOOL_CY - h / 2, w, h, 12, 'rgba(246,241,231,0.55)');
    out[it[0]] = { x: cx - w / 2, y: TOOL_CY - h / 2, w: w, h: h };
  }
  return out;
}

// ───────── 数字键盘：1~9，下面一行小字是这个数还剩几个没填 ─────────
function pad(R, st) {
  var out = [], gap = 4, w = (SIZE - 8 * gap) / 9, h = 58;
  for (var d = 1; d <= 9; d++) {
    var cx = BX + w / 2 + (d - 1) * (w + gap), left = 9 - st.p.used(d), done = left <= 0;
    R.rrect(cx - w / 2, PAD_CY - h / 2, w, h, 10, done ? '#ebe5d8' : C.btn);
    R.text(String(d), cx, PAD_CY - 7, st.notes ? 20 : 26, done ? '#c2baa9' : st.notes ? C.note : C.user, 'center', 'bold');
    if (!done) R.text(String(left), cx, PAD_CY + 18, 11, '#9a927f', 'center');   // 填满 9 个：整个键变灰，不再显示数字
    out[d] = { x: cx - w / 2, y: PAD_CY - h / 2, w: w, h: h };
  }
  return out;
}

// ───────── 菜单：四档难度 + 继续上局 ─────────
function menu(R, st) {
  var out = {}, y = B.hudTop + 40;
  R.text('数独', W / 2, y, 40, C.given, 'center', 'bold');
  R.text('每题都是唯一解 · 挖洞后用求解器校验', W / 2, y + 40, 13, '#8a8273');
  // 小装饰：3×3 的迷你宫格
  var gx = W / 2 - 39, gy = y + 66, sample = [5, 0, 3, 0, 7, 0, 9, 0, 1];
  for (var k = 0; k < 9; k++) {
    var x = gx + (k % 3) * 26, yy = gy + ((k / 3) | 0) * 26;
    R.rrect(x, yy, 24, 24, 5, k === 4 ? '#ffd8a8' : '#ffffff');
    if (sample[k]) R.text(String(sample[k]), x + 12, yy + 12.5, 14, k === 4 ? C.accent : C.given, 'center', 'bold');
  }
  y = gy + 100;
  var cw = W - 56, ch = 62;
  Gen.DIFFS.forEach(function (D, i) {
    var cy = y + i * (ch + 12), best = st.best[D.key], played = st.stats[D.key] || 0;
    R.rrect(28, cy, cw, ch, 16, '#ffffff');
    R.rrect(40, cy + 17, 28, 28, 8, ['#8ce99a', '#74c0fc', '#ffc078', '#ff8787'][i]);
    R.text(String(i + 1), 54, cy + 31.5, 16, '#ffffff', 'center', 'bold');
    R.text(D.name, 80, cy + 22, 19, C.given, 'left', 'bold');
    R.text('约 ' + D.clues + ' 个提示数', 80, cy + 44, 12, '#8a8273', 'left');
    R.text(best ? '最佳 ' + mmss(best) : '尚未完成', 28 + cw - 16, cy + 22, 15, best ? C.accent : '#b0a898', 'right', 'bold');
    R.text('完成 ' + played + ' 局', 28 + cw - 16, cy + 44, 12, '#8a8273', 'right');
    out[i] = { x: 28, y: cy, w: cw, h: ch };
  });
  y += 4 * (ch + 12) + 6;
  if (st.saved) {
    out.resume = R.button('继续上局 · ' + Gen.DIFFS[st.saved.diff].name + ' · ' + mmss(st.saved.time || 0), W / 2, y + 24, cw, 48, C.accent, '#ffffff');
  }
  R.text('键盘：1~9 填数 · 0/退格 擦除 · 方向键移动 · N 笔记 · Z 撤销 · H 提示', W / 2, Math.min(H - 20, y + 70), 10, '#a39b8b');
  return out;
}

// ───────── 出题中 ─────────
function generating(R, g) {
  var cy = H / 2 - 20, w = 220;
  R.text('正在出题…', W / 2, cy - 40, 22, C.given, 'center', 'bold');
  R.text(Gen.DIFFS[g.diff].name + ' · 挖洞 ' + (81 - g.clues) + ' 格 · 校验 ' + g.checks + ' 次', W / 2, cy - 8, 13, '#8a8273');
  R.rrect(W / 2 - w / 2, cy + 18, w, 10, 5, '#e6dfcf');
  R.rrect(W / 2 - w / 2, cy + 18, Math.max(10, w * g.progress()), 10, 5, C.accent);
}

module.exports = { L: L, C: C, hud: hud, board: board, tools: tools, pad: pad, menu: menu, generating: generating, mmss: mmss };
