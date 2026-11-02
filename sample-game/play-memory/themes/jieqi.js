// 卡面「二十四节气」：节气名 + 季节色条 + 一个小图形（春芽、夏日、秋叶、冬雪）。一局从 24 个里抽需要的对数。
var SEASON = {
  spring: { color: '#43a047', soft: '#e8f5e9' }, summer: { color: '#ef6c00', soft: '#fff3e0' },
  autumn: { color: '#c0841a', soft: '#fff8e1' }, winter: { color: '#1e88e5', soft: '#e3f2fd' }
};
var NAMES = [
  ['立春', 'spring'], ['雨水', 'spring'], ['惊蛰', 'spring'], ['春分', 'spring'], ['清明', 'spring'], ['谷雨', 'spring'],
  ['立夏', 'summer'], ['小满', 'summer'], ['芒种', 'summer'], ['夏至', 'summer'], ['小暑', 'summer'], ['大暑', 'summer'],
  ['立秋', 'autumn'], ['处暑', 'autumn'], ['白露', 'autumn'], ['秋分', 'autumn'], ['寒露', 'autumn'], ['霜降', 'autumn'],
  ['立冬', 'winter'], ['小雪', 'winter'], ['大雪', 'winter'], ['冬至', 'winter'], ['小寒', 'winter'], ['大寒', 'winter']
];
var items = NAMES.map(function (n, i) { return { key: n[0], label: n[0], season: n[1], order: i }; });

function sprout(R, x, y, s, c) {
  R.rect(x - s * 0.06, y - s * 0.2, s * 0.12, s * 0.5, c);
  R.ellipse(x - s * 0.22, y - s * 0.22, s * 0.24, s * 0.1, -0.5, c);
  R.ellipse(x + s * 0.22, y - s * 0.3, s * 0.24, s * 0.1, 0.5, c);
}
function sun(R, x, y, s, c) {
  R.circle(x, y, s * 0.24, c);
  for (var i = 0; i < 8; i++) {
    var a = i * Math.PI / 4;
    R.line([[x + Math.cos(a) * s * 0.34, y + Math.sin(a) * s * 0.34], [x + Math.cos(a) * s * 0.46, y + Math.sin(a) * s * 0.46]], s * 0.08, c, false);
  }
}
function leaf(R, x, y, s, c) {
  R.ellipse(x, y, s * 0.4, s * 0.2, -0.7, c);
  R.line([[x - s * 0.3, y + s * 0.28], [x + s * 0.26, y - s * 0.24]], s * 0.05, '#ffffff', false);
}
function snow(R, x, y, s, c) {
  for (var i = 0; i < 3; i++) {
    var a = i * Math.PI / 3;
    R.line([[x - Math.cos(a) * s * 0.42, y - Math.sin(a) * s * 0.42], [x + Math.cos(a) * s * 0.42, y + Math.sin(a) * s * 0.42]], s * 0.07, c, true);
  }
  R.circle(x, y, s * 0.1, c);
}
var ICON = { spring: sprout, summer: sun, autumn: leaf, winter: snow };

module.exports = {
  id: 'jieqi', name: '节气', back: ['#66bb6a', '#2e7d32'], items: items,
  draw: function (R, it, cx, cy, w, h) {
    var se = SEASON[it.season], s = Math.min(w, h);
    R.rrect(cx - w * 0.42, cy - h * 0.42, w * 0.84, h * 0.84, s * 0.1, se.soft);
    ICON[it.season](R, cx, cy - h * 0.16, s * 0.5, se.color);
    R.text(it.label, cx, cy + h * 0.2, Math.max(12, Math.round(s * 0.26)), se.color, 'center', 'bold');
  }
};
