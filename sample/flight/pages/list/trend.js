// 列表页的「15 天低价走势」：canvas type="2d" 折线，当前日期高亮，点一下切到那天
const D = require('../../utils/date');
const F = require('../../utils/flights');

const PAD_L = 14, PAD_R = 14, PAD_T = 26, PAD_B = 22;

function draw(page, sel, from, to, date, onTip) {
  const days = [];
  for (let i = 0; i < 15; i++) {
    const d = D.addDays(D.today(), i);
    days.push({ d, p: F.lowPrice(from, to, d) });
  }
  page._trendDays = days;
  page.createSelectorQuery().select(sel).fields({ node: true, size: true, rect: true }).exec((res) => {
    const r = res && res[0];
    if (!r || !r.node) return;
    page._trendRect = r;
    const canvas = r.node, w = r.width, h = r.height;
    const dpr = (wx.getWindowInfo ? wx.getWindowInfo().pixelRatio : 2) || 2;
    canvas.width = w * dpr; canvas.height = h * dpr;
    const ctx = canvas.getContext('2d');
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);
    const ps = days.map(x => x.p);
    const max = Math.max.apply(null, ps), min = Math.min.apply(null, ps), span = Math.max(1, max - min);
    const X = i => PAD_L + (w - PAD_L - PAD_R) * i / (days.length - 1);
    const Y = p => PAD_T + (h - PAD_T - PAD_B) * (1 - (p - min) / span);
    // 面积渐变
    const g = ctx.createLinearGradient(0, PAD_T, 0, h - PAD_B);
    g.addColorStop(0, 'rgba(31,111,255,0.28)'); g.addColorStop(1, 'rgba(31,111,255,0)');
    ctx.beginPath();
    ctx.moveTo(X(0), h - PAD_B);
    days.forEach((x, i) => ctx.lineTo(X(i), Y(x.p)));
    ctx.lineTo(X(days.length - 1), h - PAD_B);
    ctx.closePath(); ctx.fillStyle = g; ctx.fill();
    // 折线
    ctx.beginPath();
    days.forEach((x, i) => (i ? ctx.lineTo(X(i), Y(x.p)) : ctx.moveTo(X(i), Y(x.p))));
    ctx.strokeStyle = '#1f6fff'; ctx.lineWidth = 2; ctx.lineJoin = 'round'; ctx.stroke();
    // 点、最低价、当前日
    ctx.font = '10px sans-serif'; ctx.textAlign = 'center';
    let minI = ps.indexOf(min);
    days.forEach((x, i) => {
      const cur = x.d === date;
      ctx.beginPath(); ctx.arc(X(i), Y(x.p), cur ? 4.5 : 2.5, 0, Math.PI * 2);
      ctx.fillStyle = cur ? '#ff6a00' : i === minI ? '#12b76a' : '#1f6fff'; ctx.fill();
      if (cur || i === minI) {
        ctx.fillStyle = cur ? '#ff6a00' : '#12b76a';
        ctx.fillText('¥' + x.p, X(i), Y(x.p) - 9);
      }
      if (i % 2 === 0) {
        ctx.fillStyle = '#8a93a6';
        ctx.textAlign = i === 0 ? 'left' : i === days.length - 1 ? 'right' : 'center';
        ctx.fillText(D.mdShort(x.d), i === 0 ? 0 : i === days.length - 1 ? w : X(i), h - 6);
        ctx.textAlign = 'center';
      }
    });
    const best = days[minI];
    if (onTip) onTip('15 天内最低 ¥' + best.p + '（' + D.md(best.d) + ' ' + D.weekLabel(best.d) + '）');
  });
}

// 点击位置 → 最近的那一天
function hit(page, e) {
  const r = page._trendRect, days = page._trendDays;
  if (!r || !days) return '';
  const x = (e.detail && e.detail.x !== undefined ? e.detail.x : 0) - r.left;
  const step = (r.width - PAD_L - PAD_R) / (days.length - 1);
  const i = Math.max(0, Math.min(days.length - 1, Math.round((x - PAD_L) / step)));
  return days[i].d;
}

module.exports = { draw, hit };
