// 生长曲线：canvas type="2d"，按 dpr 放大画布像素再 scale 回逻辑坐标，线条不发虚
const WHO = require('../../utils/who.js');

const STYLE = {
  weight: { unit: 'kg', step: 1, color: '#ff7a8a' },
  height: { unit: 'cm', step: 5, color: '#4a9ef0' }
};

function draw(canvas, w, h, opt) {
  const dpr = opt.dpr || 2;
  canvas.width = Math.round(w * dpr);
  canvas.height = Math.round(h * dpr);
  const ctx = canvas.getContext('2d');
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, w, h);

  const st = STYLE[opt.metric];
  const pad = { l: 34, r: 30, t: 12, b: 26 };
  const xmax = opt.xmax;
  const rows = WHO.TABLE[opt.sex === 'girl' ? 'girl' : 'boy'][opt.metric];
  let ymin = Math.floor(WHO.at(rows[0], 0) / st.step) * st.step;
  let ymax = Math.ceil(WHO.at(rows[4], xmax) / st.step) * st.step;
  opt.points.forEach((p) => { ymin = Math.min(ymin, Math.floor(p.v / st.step) * st.step); ymax = Math.max(ymax, Math.ceil(p.v / st.step) * st.step); });
  const X = (m) => pad.l + (w - pad.l - pad.r) * m / xmax;
  const Y = (v) => pad.t + (h - pad.t - pad.b) * (1 - (v - ymin) / (ymax - ymin));

  // 网格与刻度
  ctx.strokeStyle = '#f1ecee';
  ctx.lineWidth = 1;
  ctx.fillStyle = '#a69ca0';
  ctx.font = '10px sans-serif';
  const ystep = (ymax - ymin) / st.step > 12 ? st.step * 2 : st.step;
  ctx.textAlign = 'right';
  ctx.textBaseline = 'middle';
  for (let v = ymin; v <= ymax + 1e-6; v += ystep) {
    ctx.beginPath(); ctx.moveTo(pad.l, Y(v)); ctx.lineTo(w - pad.r, Y(v)); ctx.stroke();
    ctx.fillText(String(v), pad.l - 6, Y(v));
  }
  const xstep = xmax > 12 ? 3 : 2;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'top';
  for (let m = 0; m <= xmax; m += xstep) {
    ctx.beginPath(); ctx.moveTo(X(m), pad.t); ctx.lineTo(X(m), h - pad.b); ctx.stroke();
    ctx.fillText(m + '月', X(m), h - pad.b + 6);
  }

  // 百分位带：P3~P97 浅、P15~P85 深
  const steps = [];
  for (let m = 0; m <= xmax + 1e-6; m += 0.25) steps.push(m);
  const band = (lo, hi, color) => {
    ctx.beginPath();
    steps.forEach((m, i) => { const y = Y(WHO.at(rows[hi], m)); i ? ctx.lineTo(X(m), y) : ctx.moveTo(X(m), y); });
    for (let i = steps.length - 1; i >= 0; i--) ctx.lineTo(X(steps[i]), Y(WHO.at(rows[lo], steps[i])));
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
  };
  band(0, 4, 'rgba(47,174,147,0.08)');
  band(1, 3, 'rgba(47,174,147,0.14)');
  const line = (row, color, width) => {
    ctx.beginPath();
    steps.forEach((m, i) => { const y = Y(WHO.at(row, m)); i ? ctx.lineTo(X(m), y) : ctx.moveTo(X(m), y); });
    ctx.strokeStyle = color;
    ctx.lineWidth = width;
    ctx.stroke();
  };
  rows.forEach((row, i) => line(row, i === 2 ? '#2fae93' : 'rgba(47,174,147,0.45)', i === 2 ? 1.5 : 1));
  ctx.fillStyle = '#2fae93';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  WHO.P.forEach((p, i) => ctx.fillText(p, w - pad.r + 3, Y(WHO.at(rows[i], xmax))));

  // 宝宝的测量点
  if (opt.points.length) {
    ctx.beginPath();
    opt.points.forEach((p, i) => { i ? ctx.lineTo(X(p.m), Y(p.v)) : ctx.moveTo(X(p.m), Y(p.v)); });
    ctx.strokeStyle = st.color;
    ctx.lineWidth = 2;
    ctx.stroke();
    opt.points.forEach((p) => {
      ctx.beginPath();
      ctx.arc(X(p.m), Y(p.v), 3.5, 0, Math.PI * 2);
      ctx.fillStyle = '#ffffff';
      ctx.fill();
      ctx.lineWidth = 2;
      ctx.strokeStyle = st.color;
      ctx.stroke();
    });
  }
  ctx.fillStyle = '#a69ca0';
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  ctx.fillText('单位 ' + st.unit, pad.l + 6, pad.t + 4);
}

module.exports = { draw, STYLE };
