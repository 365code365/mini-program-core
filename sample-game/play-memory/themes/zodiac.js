// 卡面「生肖」：十二生肖 + 六个吉祥字，印章样式（圆章 / 方章交替，各自的底色）。
var LIST = [
  ['鼠', '#8d6e63'], ['牛', '#6d4c41'], ['虎', '#f57c00'], ['兔', '#ec407a'], ['龙', '#d32f2f'], ['蛇', '#388e3c'],
  ['马', '#5d4037'], ['羊', '#7e57c2'], ['猴', '#ff8f00'], ['鸡', '#e53935'], ['狗', '#795548'], ['猪', '#f06292'],
  ['福', '#c62828'], ['禄', '#2e7d32'], ['寿', '#ef6c00'], ['喜', '#d81b60'], ['和', '#1565c0'], ['安', '#00897b']
];
var items = LIST.map(function (x, i) { return { key: x[0], label: x[0], color: x[1], round: i % 2 === 0, lucky: i >= 12 }; });

module.exports = {
  id: 'zodiac', name: '生肖', back: ['#ef5350', '#b71c1c'], items: items,
  draw: function (R, it, cx, cy, w, h) {
    var s = Math.min(w, h), r = s * 0.36;
    if (it.round) { R.circle(cx, cy, r, it.color); R.strokeCircle(cx, cy, r * 0.84, Math.max(1.5, s * 0.03), 'rgba(255,255,255,0.8)'); }
    else { R.rrect(cx - r, cy - r, r * 2, r * 2, r * 0.18, it.color); R.strokeRect(cx - r * 0.8, cy - r * 0.8, r * 1.6, r * 1.6, Math.max(1.5, s * 0.03), 'rgba(255,255,255,0.8)'); }
    R.text(it.label, cx, cy + 1, Math.round(r * 1.05), '#ffffff', 'center', 'bold');
    if (it.lucky) R.text('吉', cx + r * 0.95, cy - r * 0.95, Math.max(10, Math.round(s * 0.14)), it.color, 'center', 'bold');
  }
};
