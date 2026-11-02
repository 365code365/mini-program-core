// 卡面主题。「几何」在主包里（默认，开局不用等）；「节气」「生肖」放在分包 themes/（game.json 的 subpackages），
// 第一次选它们时 wx.loadSubpackage 加载，分包入口 themes/game.js 把主题登记进 GameGlobal.__memThemes。
// 主题的形状：{ id, name, back: [背面渐变起, 止], items: [≥18 项], draw(R, item, cx, cy, w, h) }
var reg = GameGlobal.__memThemes = GameGlobal.__memThemes || {};
var META = [
  { id: 'geo', name: '几何', pkg: null },
  { id: 'jieqi', name: '节气', pkg: 'themes' },
  { id: 'zodiac', name: '生肖', pkg: 'themes' }
];

// ───────── 几何：6 种形状 × 3 种颜色 = 18 张 ─────────
var SHAPES = ['circle', 'triangle', 'square', 'diamond', 'star', 'hexagon'];
var HUES = ['#e53935', '#1e88e5', '#43a047'];
var items = [];
SHAPES.forEach(function (s) { HUES.forEach(function (c) { items.push({ key: s + c, shape: s, color: c }); }); });
function star(R, cx, cy, ro, ri, color) {
  var p = [];
  for (var i = 0; i < 10; i++) { var a = -Math.PI / 2 + Math.PI * i / 5, r = i % 2 ? ri : ro; p.push([cx + Math.cos(a) * r, cy + Math.sin(a) * r]); }
  for (var k = 0; k < 10; k++) R.poly([[cx, cy], p[k], p[(k + 1) % 10]], color);
}
function ngon(R, cx, cy, r, n, rot, color) {
  var p = [];
  for (var i = 0; i < n; i++) { var a = rot + Math.PI * 2 * i / n; p.push([cx + Math.cos(a) * r, cy + Math.sin(a) * r]); }
  R.poly(p, color);
}
reg.geo = {
  id: 'geo', name: '几何', back: ['#5c6bc0', '#3949ab'], items: items,
  draw: function (R, it, cx, cy, w, h) {
    var r = Math.min(w, h) * 0.3, c = it.color;
    if (it.shape === 'circle') R.circle(cx, cy, r, c);
    else if (it.shape === 'triangle') ngon(R, cx, cy + r * 0.15, r * 1.12, 3, -Math.PI / 2, c);
    else if (it.shape === 'square') R.rrect(cx - r * 0.85, cy - r * 0.85, r * 1.7, r * 1.7, r * 0.2, c);
    else if (it.shape === 'diamond') ngon(R, cx, cy, r * 1.1, 4, -Math.PI / 2, c);
    else if (it.shape === 'star') star(R, cx, cy, r * 1.15, r * 0.48, c);
    else ngon(R, cx, cy, r, 6, 0, c);
    R.circle(cx - r * 0.3, cy - r * 0.3, r * 0.16, 'rgba(255,255,255,0.45)');
  }
};

var loading = null;     // { id, progress }
// 拿到主题（要分包就先加载）。cb(theme, err)：加载失败回落到「几何」并给出原因
function ensure(id, cb) {
  if (reg[id]) return cb(reg[id], null);
  var meta = META.filter(function (m) { return m.id === id; })[0];
  if (!meta || !meta.pkg || !wx.loadSubpackage) return cb(reg.geo, '这个主题不可用');
  loading = { id: id, progress: 0 };
  var task = wx.loadSubpackage({
    name: meta.pkg,
    success: function () { loading = null; cb(reg[id] || reg.geo, reg[id] ? null : '分包里没有这个主题'); },
    fail: function (e) { loading = null; cb(reg.geo, (e && e.errMsg) || '分包加载失败'); }
  });
  if (task && task.onProgressUpdate) task.onProgressUpdate(function (p) { if (loading) loading.progress = p.progress; });
}
module.exports = { META: META, ensure: ensure, get: function (id) { return reg[id]; }, loading: function () { return loading; },
  loaded: function (id) { return !!reg[id]; } };
