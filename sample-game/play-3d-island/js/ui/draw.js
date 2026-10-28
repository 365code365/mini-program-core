// 界面绘制小工具（画在视图的离屏 2D 画布上）：主题色、面板框、按钮、资源条、费用行、进度条、红点、换行文字。
// 风格：奶油底 + 深棕描字 + 糖果色按钮（上亮下暗 + 底部厚边 = 「能按」的感觉），圆角统一 14。
var E = require('../core/economy.js');
var T = {
  bg: '#fff8ec', bg2: '#ffefd6', line: '#e8cfa6', ink: '#5a3a1a', ink2: '#9a7a55', mute: '#c2ab8a',
  green: ['#7be37a', '#34b24a', '#1f7a30'], blue: ['#7cc8ff', '#2f8fe8', '#1a5fa8'], orange: ['#ffcf5c', '#ff9f1c', '#c46a00'],
  red: ['#ff8a8a', '#ef4b4b', '#a82020'], purple: ['#d4a6ff', '#9b5de5', '#5e2ca5'], gray: ['#e5e5e5', '#b8b8b8', '#8a8a8a'],
  gold: '#ffc93c'
};
function rr(c, x, y, w, h, r) {
  r = Math.min(r, w / 2, h / 2);
  c.beginPath(); c.moveTo(x + r, y); c.arcTo(x + w, y, x + w, y + h, r); c.arcTo(x + w, y + h, x, y + h, r); c.arcTo(x, y + h, x, y, r); c.arcTo(x, y, x + w, y, r); c.closePath();
}
function font(c, size, bold) { c.font = (bold ? 'bold ' : '') + size + 'px sans-serif'; }
function text(c, s, x, y, size, color, align, bold) {
  font(c, size, bold); c.fillStyle = color || T.ink; c.textAlign = align || 'left'; c.textBaseline = 'middle'; c.fillText(String(s), x, y);
}
// 白描边的字（标题、按钮上的字）：8 个方向偏移写一遍描边色（本引擎 strokeText 很慢）
function textOutlined(c, s, x, y, size, color, stroke, align, w) {
  font(c, size, true); c.textAlign = align || 'center'; c.textBaseline = 'middle';
  c.fillStyle = stroke; var o = w || Math.max(1.5, size / 12);
  for (var k = 0; k < 8; k++) { var a = k * Math.PI / 4; c.fillText(String(s), x + Math.cos(a) * o, y + Math.sin(a) * o); }
  c.fillStyle = color; c.fillText(String(s), x, y);
}
function wrap(c, s, x, y, maxW, size, color, lineH, maxLines) {
  font(c, size); c.fillStyle = color || T.ink2; c.textAlign = 'left'; c.textBaseline = 'middle';
  var line = '', n = 0;
  for (var i = 0; i < s.length; i++) {
    var ch = s[i];
    if (ch === '\n' || c.measureText(line + ch).width > maxW) {
      // 避头尾：左括号 / 左引号不留在行尾，挪到下一行开头（否则「…钻石（」换行「共 3300）」）
      var carry = '';
      while (ch !== '\n' && line.length > 1 && '（(「《“【'.indexOf(line[line.length - 1]) >= 0) { carry = line[line.length - 1] + carry; line = line.slice(0, -1); }
      c.fillText(line, x, y + n * lineH); n++; line = ch === '\n' ? '' : carry + ch;
      if (maxLines && n >= maxLines) return n;
    } else line += ch;
  }
  if (line) { c.fillText(line, x, y + n * lineH); n++; }
  return n;
}
// 面板：外框深边 + 奶油底 + 顶部标题带（丝带色）
function panel(c, w, h, title, ribbon) {
  c.fillStyle = 'rgba(80,45,10,0.28)'; rr(c, 4, 8, w - 8, h - 8, 22); c.fill();
  c.fillStyle = '#a7743f'; rr(c, 0, 0, w, h - 4, 22); c.fill();
  c.fillStyle = T.bg; rr(c, 5, 5, w - 10, h - 14, 18); c.fill();
  if (title) {
    var col = ribbon || T.orange;
    var g = c.createLinearGradient(0, 8, 0, 52); g.addColorStop(0, col[0]); g.addColorStop(1, col[1]);
    c.fillStyle = g; rr(c, 5, 5, w - 10, 50, 18); c.fill(); c.fillRect(5, 30, w - 10, 25);
    c.fillStyle = 'rgba(255,255,255,0.25)'; c.fillRect(5, 32, w - 10, 3);
    textOutlined(c, title, w / 2, 31, 21, '#fff', col[2]);
  }
}
// 按钮：style = T.green / blue / …；disabled 变灰；返回按钮矩形（调用方 v.hit）
function button(c, x, y, w, h, label, style, opts) {
  opts = opts || {};
  var s = opts.disabled ? T.gray : style || T.green;
  c.fillStyle = s[2]; rr(c, x, y + 3, w, h - 3, h / 2.4); c.fill();
  var g = c.createLinearGradient(0, y, 0, y + h - 4); g.addColorStop(0, s[0]); g.addColorStop(1, s[1]);
  c.fillStyle = g; rr(c, x, y, w, h - 5, h / 2.4); c.fill();
  c.fillStyle = 'rgba(255,255,255,0.35)'; rr(c, x + 6, y + 3, w - 12, (h - 5) * 0.32, h / 5); c.fill();
  var ic = opts.icon, size = opts.size || Math.round(h * 0.4);
  if (ic && opts.icons) {
    font(c, size, true);
    var tw = c.measureText(label).width, iw = size * 1.4, x0 = x + (w - tw - iw - 2) / 2;
    opts.icons.draw(c, ic, x0, y + (h - 5) / 2 - iw / 2, iw);
    textOutlined(c, label, x0 + iw + 2 + tw / 2, y + (h - 5) / 2 + 1, size, '#fff', s[2]);
  } else textOutlined(c, label, x + w / 2, y + (h - 5) / 2 + 1, size, '#fff', s[2]);
  return { x: x, y: y, w: w, h: h };
}
// 费用 / 奖励一行：图标 + 数字；have 给了就按够不够标红
function costRow(c, icons, cost, x, y, size, have, align) {
  var items = Object.keys(cost), gap = 10, parts = [];
  font(c, size, true);
  items.forEach(function (k) { parts.push({ k: k, s: E.fmt(cost[k]), w: c.measureText(E.fmt(cost[k])).width + size * 1.25 + 4 }); });
  var total = parts.reduce(function (a, p) { return a + p.w + gap; }, -gap);
  var px = align === 'center' ? x - total / 2 : align === 'right' ? x - total : x;
  parts.forEach(function (p) {
    icons.draw(c, p.k, px, y - size * 0.65, size * 1.3);
    var short = have && (have[p.k] || 0) < cost[p.k];
    text(c, p.s, px + size * 1.3 + 3, y, size, short ? '#e5383b' : T.ink, 'left', true);
    px += p.w + gap;
  });
  return total;
}
function bar(c, x, y, w, h, k, col) {
  c.fillStyle = 'rgba(90,58,26,0.18)'; rr(c, x, y, w, h, h / 2); c.fill();
  if (k > 0) { var g = c.createLinearGradient(0, y, 0, y + h); g.addColorStop(0, (col || T.green)[0]); g.addColorStop(1, (col || T.green)[1]); c.fillStyle = g; rr(c, x, y, Math.max(h, w * Math.min(1, k)), h, h / 2); c.fill(); }
  c.fillStyle = 'rgba(255,255,255,0.35)'; rr(c, x + 3, y + 2, Math.max(0, w * Math.min(1, k) - 6), h * 0.3, h / 4); c.fill();
}
function badge(c, x, y, n) {
  c.fillStyle = '#ff3b30'; c.beginPath(); c.arc(x, y, 9, 0, Math.PI * 2); c.fill();
  c.lineWidth = 2; c.strokeStyle = '#fff'; c.stroke();
  if (n) text(c, n > 99 ? '99+' : n, x, y + 0.5, 11, '#fff', 'center', true);
}
// 卡片（列表里的一行）
function card(c, x, y, w, h, hi) {
  c.fillStyle = hi ? '#fff3c4' : '#fffdf8'; rr(c, x, y, w, h, 14); c.fill();
  c.lineWidth = 2; c.strokeStyle = hi ? '#ffcf5c' : T.line; c.stroke();
}
function closeBtn(c, icons, v) { icons.draw(c, 'close', v.w - 46, 9, 38); v.hit('close', v.w - 50, 4, 48, 48); }
function tag(c, x, y, s, col) {
  font(c, 11, true); var w = c.measureText(s).width + 12;
  c.fillStyle = col || '#ff3b30'; rr(c, x, y, w, 18, 9); c.fill();
  text(c, s, x + w / 2, y + 9.5, 11, '#fff', 'center', true);
  return w;
}

module.exports = { T: T, rr: rr, font: font, text: text, textOutlined: textOutlined, wrap: wrap, panel: panel, button: button, costRow: costRow,
  bar: bar, badge: badge, card: card, closeBtn: closeBtn, tag: tag };
