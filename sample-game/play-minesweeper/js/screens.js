// 棋盘以外的画面：顶栏（剩余雷数 / 笑脸重开 / 计时）、底栏（菜单 / 点按模式 / 适应屏幕）、
// 选关菜单、胜负面板。都画在主画布的 2D 上下文上，返回命中框给 game.js 判点按。
var B = require('./base.js');
var Board = require('./board.js');
var W = B.W, H = B.H;
var ORDER = ['easy', 'mid', 'hard'];

function mmss(t) {
  t = Math.floor(t);
  var m = Math.floor(t / 60), s = t % 60;
  return (m < 10 ? '0' : '') + m + ':' + (s < 10 ? '0' : '') + s;
}

function pill(ctx, x, y, w, h, bg) { B.rrect(x, y, w, h, h / 2, ctx); ctx.fillStyle = bg; ctx.fill(); }

function miniMine(ctx, cx, cy, r) {
  ctx.strokeStyle = '#ffffff'; ctx.lineWidth = 2; ctx.lineCap = 'round';
  ctx.beginPath();
  for (var k = 0; k < 4; k++) {
    var a = k * Math.PI / 4, dx = Math.cos(a) * r * 1.5, dy = Math.sin(a) * r * 1.5;
    ctx.moveTo(cx - dx, cy - dy); ctx.lineTo(cx + dx, cy + dy);
  }
  ctx.stroke();
  ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fillStyle = '#ffffff'; ctx.fill();
}

// 笑脸：进行中微笑、按住格子时「O」嘴、踩雷叉眼、胜利墨镜
function face(ctx, cx, cy, r, mood) {
  ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fillStyle = '#ffd43b'; ctx.fill();
  ctx.strokeStyle = '#5c3d00'; ctx.lineWidth = 2; ctx.lineCap = 'round';
  var ex = r * 0.38, ey = r * 0.25;
  if (mood === 'lost') {
    ctx.beginPath();
    [-1, 1].forEach(function (s) {
      var x = cx + s * ex, y = cy - ey;
      ctx.moveTo(x - 3, y - 3); ctx.lineTo(x + 3, y + 3); ctx.moveTo(x + 3, y - 3); ctx.lineTo(x - 3, y + 3);
    });
    ctx.stroke();
    ctx.beginPath(); ctx.arc(cx, cy + r * 0.55, r * 0.32, Math.PI * 1.15, Math.PI * 1.85); ctx.stroke();
    return;
  }
  if (mood === 'won') {
    ctx.fillStyle = '#212529';
    ctx.fillRect(cx - r * 0.75, cy - ey - 3, r * 1.5, 3);
    B.rrect(cx - ex - 5, cy - ey - 3, 10, 7, 3, ctx); ctx.fill();
    B.rrect(cx + ex - 5, cy - ey - 3, 10, 7, 3, ctx); ctx.fill();
  } else {
    ctx.fillStyle = '#5c3d00';
    ctx.beginPath(); ctx.arc(cx - ex, cy - ey, 2.2, 0, Math.PI * 2); ctx.arc(cx + ex, cy - ey, 2.2, 0, Math.PI * 2); ctx.fill();
  }
  ctx.beginPath();
  if (mood === 'press') ctx.arc(cx, cy + r * 0.32, r * 0.18, 0, Math.PI * 2);
  else ctx.arc(cx, cy + r * 0.05, r * 0.45, Math.PI * 0.2, Math.PI * 0.8);
  ctx.stroke();
}

// 顶栏：返回 { face } 命中框
function hud(ctx, s) {
  var y = B.hudTop, h = 40, cy = y + h / 2;
  pill(ctx, 12, y + 4, 96, h - 8, 'rgba(255,255,255,0.10)');
  miniMine(ctx, 32, cy, 6);
  B.text(String(s.board.mines - s.board.flags), 50, cy, 18, '#ffffff', 'left', 'bold');
  pill(ctx, W - 108, y + 4, 96, h - 8, 'rgba(255,255,255,0.10)');
  B.text(mmss(s.time), W - 60, cy, 18, '#ffffff', 'center', 'bold');
  var mood = s.board.state === 'won' ? 'won' : s.board.state === 'lost' ? 'lost' : s.pressing ? 'press' : 'play';
  face(ctx, W / 2, cy, 17, mood);
  B.text(Board.LEVELS[s.board.level].name, W / 2 - 34, cy, 13, '#9fb3d9', 'right');
  return { face: { x: W / 2 - 24, y: y, w: 48, h: h } };
}

// 底栏：返回 { menu, mode, fit }
function bar(ctx, s, cy) {
  var out = {};
  out.menu = B.button('菜单', 52, cy, 80, 40, 'rgba(255,255,255,0.12)', '#ffffff', 15);
  var flagMode = s.mode === 'flag';
  out.mode = B.button(flagMode ? '点按：插旗' : '点按：翻开', W / 2, cy, 132, 40, flagMode ? '#ff6b6b' : '#4dabf7', '#ffffff', 15);
  out.fit = B.button(s.zoomed ? '全览' : '放大', W - 52, cy, 80, 40, 'rgba(255,255,255,0.12)', '#ffffff', 15);
  B.text('长按插旗 · 双击数字开周围 · 双指缩放拖动', W / 2, cy - 34, 11, 'rgba(255,255,255,0.45)');
  return out;
}

// 选关菜单：返回 { easy, mid, hard, resume }
function menu(ctx, s) {
  var out = {};
  var g = ctx.createLinearGradient(0, 0, 0, H);
  g.addColorStop(0, '#1b2a4a'); g.addColorStop(1, '#0e1628');
  ctx.fillStyle = g; ctx.fillRect(0, 0, W, H);
  var y = B.hudTop + 30;
  // 标题旁画一颗大雷
  ctx.save(); ctx.translate(W / 2 - 62, y); miniMine(ctx, 0, 0, 13); ctx.restore();
  B.text('扫雷', W / 2 + 14, y, 36, '#ffffff', 'center', 'bold');
  B.text('首次点击必不是雷 · 空白自动展开', W / 2, y + 40, 13, '#9fb3d9');
  y += 78;
  var cw = W - 48, ch = 84;
  ORDER.forEach(function (k, idx) {
    var L = Board.LEVELS[k], best = s.data.best[k], st = s.data.stats[k] || { played: 0, won: 0 };
    var x = 24, cyy = y + idx * (ch + 14);
    B.rrect(x, cyy, cw, ch, 16, ctx); ctx.fillStyle = '#243a63'; ctx.fill();
    B.rrect(x + 8, cyy + 18, 5, ch - 36, 2.5, ctx); ctx.fillStyle = ['#51cf66', '#fcc419', '#ff6b6b'][idx]; ctx.fill();
    B.text(L.name, x + 24, cyy + 28, 22, '#ffffff', 'left', 'bold');
    B.text(L.cols + ' × ' + L.rows + ' · ' + L.mines + ' 颗雷', x + 24, cyy + 58, 13, '#9fb3d9', 'left');
    B.text(best ? '最佳 ' + mmss(best) : '尚未通关', x + cw - 20, cyy + 28, 16, best ? '#ffd43b' : '#7d8fb3', 'right', 'bold');
    B.text('胜 ' + st.won + ' / 局 ' + st.played, x + cw - 20, cyy + 58, 13, '#9fb3d9', 'right');
    out[k] = { x: x, y: cyy, w: cw, h: ch };
  });
  y += 3 * (ch + 14) + 10;
  var cur = s.data.cur;
  if (cur && Board.LEVELS[cur.level]) {
    out.resume = B.button('继续上局 · ' + Board.LEVELS[cur.level].name + ' · 已用 ' + mmss(cur.time || 0), W / 2, y + 24, cw, 48, '#4dabf7', '#ffffff', 16);
    y += 64;
  }
  B.text(s.saveLine, W / 2, Math.min(H - 24, y + 24), 11, 'rgba(255,255,255,0.4)');
  return out;
}

// 胜负面板（盖在棋盘下半部分，上半部分还能看到亮出来的雷）：返回 { again, levels }
function over(ctx, s, cy) {
  var out = {}, w = Math.min(W - 40, 320), h = 150, x = (W - w) / 2, y = cy - h / 2;
  B.rrect(x, y, w, h, 18, ctx); ctx.fillStyle = 'rgba(12,20,38,0.92)'; ctx.fill();
  var won = s.board.state === 'won';
  B.text(won ? (s.newBest ? '新纪录！' : '扫雷成功') : '踩到雷了', W / 2, y + 30, 24, won ? '#ffd43b' : '#ff8787', 'center', 'bold');
  var best = s.data.best[s.board.level];
  B.text('用时 ' + mmss(s.time) + (best ? '　最佳 ' + mmss(best) : ''), W / 2, y + 64, 15, '#dee6f5');
  out.again = B.button('再来一局', W / 2 - 72, y + h - 34, 124, 40, '#4dabf7', '#ffffff', 15);
  out.levels = B.button('换难度', W / 2 + 72, y + h - 34, 124, 40, 'rgba(255,255,255,0.14)', '#ffffff', 15);
  return out;
}

module.exports = { hud: hud, bar: bar, menu: menu, over: over, mmss: mmss, ORDER: ORDER };
