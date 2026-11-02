// 保费随年龄变化的折线图（canvas type="2d"）。pts: [{age, v}]；cur: 当前年龄；w/h: CSS 像素。
const util = require('./util.js');

function niceMax(v) {
  if (v <= 0) return 100;
  const p = Math.pow(10, Math.floor(Math.log10(v)));
  const n = v / p;
  return (n <= 1 ? 1 : n <= 2 ? 2 : n <= 5 ? 5 : 10) * p;
}

function draw(ctx, w, h, pts, cur, curV) {
  const L = 46, R = 14, T = 18, B = 28;
  const cw = w - L - R, ch = h - T - B;
  ctx.clearRect(0, 0, w, h);
  const maxV = niceMax(Math.max.apply(null, pts.map(p => p.v)) * 1.05);
  const a0 = pts[0].age, a1 = pts[pts.length - 1].age;
  const X = a => L + (a1 === a0 ? cw / 2 : (a - a0) / (a1 - a0) * cw);
  const Y = v => T + ch - v / maxV * ch;

  // 网格 + 纵轴刻度
  ctx.strokeStyle = '#eef0f5';
  ctx.lineWidth = 1;
  ctx.fillStyle = '#9aa3b2';
  ctx.font = '10px sans-serif';
  ctx.textAlign = 'right';
  ctx.textBaseline = 'middle';
  for (let i = 0; i <= 4; i++) {
    const v = maxV * i / 4, y = Y(v);
    ctx.beginPath(); ctx.moveTo(L, y); ctx.lineTo(w - R, y); ctx.stroke();
    ctx.fillText(v >= 10000 ? (v / 10000).toFixed(v % 10000 ? 1 : 0) + '万' : String(Math.round(v)), L - 6, y);
  }
  // 横轴年龄
  ctx.textAlign = 'center';
  ctx.textBaseline = 'top';
  const step = Math.max(1, Math.ceil(pts.length / 6));
  pts.forEach((p, i) => { if (i % step === 0 || i === pts.length - 1) ctx.fillText(p.age + '岁', X(p.age), T + ch + 8); });

  // 面积 + 折线
  const grad = ctx.createLinearGradient(0, T, 0, T + ch);
  grad.addColorStop(0, 'rgba(47,107,255,0.28)');
  grad.addColorStop(1, 'rgba(47,107,255,0.02)');
  ctx.beginPath();
  ctx.moveTo(X(pts[0].age), T + ch);
  pts.forEach(p => ctx.lineTo(X(p.age), Y(p.v)));
  ctx.lineTo(X(pts[pts.length - 1].age), T + ch);
  ctx.closePath();
  ctx.fillStyle = grad;
  ctx.fill();
  ctx.beginPath();
  pts.forEach((p, i) => (i ? ctx.lineTo(X(p.age), Y(p.v)) : ctx.moveTo(X(p.age), Y(p.v))));
  ctx.strokeStyle = '#2f6bff';
  ctx.lineWidth = 2;
  ctx.stroke();

  // 当前年龄的点与标签
  const cx = X(cur), cy = Y(curV);
  ctx.setLineDash([3, 3]);
  ctx.strokeStyle = '#ff8a1f';
  ctx.lineWidth = 1;
  ctx.beginPath(); ctx.moveTo(cx, T); ctx.lineTo(cx, T + ch); ctx.stroke();
  ctx.setLineDash([]);
  ctx.beginPath(); ctx.arc(cx, cy, 5, 0, Math.PI * 2);
  ctx.fillStyle = '#ffffff'; ctx.fill();
  ctx.lineWidth = 2.5; ctx.strokeStyle = '#ff8a1f'; ctx.stroke();
  const label = cur + '岁 ¥' + util.money(curV);
  ctx.font = 'bold 11px sans-serif';
  const tw = ctx.measureText(label).width + 12;
  const bx = Math.min(Math.max(cx - tw / 2, L), w - R - tw);
  const by = Math.max(cy - 30, 0);
  ctx.fillStyle = '#ff8a1f';
  ctx.fillRect(bx, by, tw, 20);
  ctx.fillStyle = '#ffffff';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(label, bx + tw / 2, by + 10);
}

module.exports = { draw };
