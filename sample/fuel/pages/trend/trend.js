const { OILS, TREND } = require('../../utils/data.js');
const store = require('../../utils/store.js');

// 油价走势：canvas type="2d" 画折线 + 渐变面积 + 最高 / 最低标注；点按画布显示那一轮的价格
Page({
  data: { oils: OILS, oil: '95#', cur: '', change: '', changeUp: false, high: '', low: '', next: TREND.next, picked: null, rows: [], canvasOk: true },

  onLoad() { this.setData({ oil: store.oilPref() }); },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 1 });
    if (this.data.oil !== store.oilPref() && OILS.indexOf(store.oilPref()) >= 0) this.setData({ oil: store.oilPref(), picked: null });
    this.summary();
  },
  onReady() { this.initCanvas(); },

  initCanvas() {
    wx.createSelectorQuery().select('#trend').fields({ node: true, size: true }).exec((res) => {
      const r = res && res[0];
      if (!r || !r.node) { this.setData({ canvasOk: false }); return; }
      const dpr = wx.getWindowInfo().pixelRatio || 2;
      r.node.width = r.width * dpr;
      r.node.height = r.height * dpr;
      this.cv = { node: r.node, ctx: r.node.getContext('2d'), w: r.width, h: r.height, dpr };
      this.draw();
    });
  },

  summary() {
    const s = TREND[this.data.oil];
    const last = s[s.length - 1], prev = s[s.length - 2];
    const d = last - prev;
    this.setData({
      cur: last.toFixed(2), change: (d >= 0 ? '+' : '') + d.toFixed(2), changeUp: d > 0,
      high: Math.max.apply(null, s).toFixed(2), low: Math.min.apply(null, s).toFixed(2),
      rows: TREND.dates.map((dt, i) => ({ d: dt, v: s[i].toFixed(2), diff: i ? (s[i] - s[i - 1]).toFixed(2) : '—' })).slice(-6).reverse()
    });
    this.draw();
  },

  geom() {
    const { w, h } = this.cv;
    const s = TREND[this.data.oil];
    const lo = Math.min.apply(null, s) - 0.05, hi = Math.max.apply(null, s) + 0.05;
    const L = 40, R = 12, T = 18, B = 26;
    const x = (i) => L + ((w - L - R) * i) / (s.length - 1);
    const y = (v) => T + ((h - T - B) * (hi - v)) / (hi - lo);
    return { s, lo, hi, L, R, T, B, x, y };
  },

  draw() {
    if (!this.cv) return;
    const { ctx, w, h, dpr } = this.cv;
    const g = this.geom();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    // 网格与纵轴刻度
    ctx.strokeStyle = '#eef1f3';
    ctx.lineWidth = 1;
    ctx.fillStyle = '#8b939b';
    ctx.font = '10px sans-serif';
    for (let k = 0; k <= 4; k++) {
      const v = g.lo + ((g.hi - g.lo) * k) / 4;
      const yy = g.y(v);
      ctx.beginPath(); ctx.moveTo(g.L, yy); ctx.lineTo(w - g.R, yy); ctx.stroke();
      ctx.fillText(v.toFixed(2), 4, yy + 3);
    }
    [0, 5, 10, 15, TREND.dates.length - 1].forEach((i) => ctx.fillText(TREND.dates[i], Math.min(g.x(i) - 14, w - 30), h - 8));
    // 面积
    const grad = ctx.createLinearGradient(0, g.T, 0, h - g.B);
    grad.addColorStop(0, 'rgba(19,168,116,0.35)');
    grad.addColorStop(1, 'rgba(19,168,116,0.02)');
    ctx.beginPath();
    g.s.forEach((v, i) => (i ? ctx.lineTo(g.x(i), g.y(v)) : ctx.moveTo(g.x(i), g.y(v))));
    ctx.lineTo(g.x(g.s.length - 1), h - g.B);
    ctx.lineTo(g.x(0), h - g.B);
    ctx.closePath();
    ctx.fillStyle = grad;
    ctx.fill();
    // 折线
    ctx.beginPath();
    g.s.forEach((v, i) => (i ? ctx.lineTo(g.x(i), g.y(v)) : ctx.moveTo(g.x(i), g.y(v))));
    ctx.strokeStyle = '#13a874';
    ctx.lineWidth = 2;
    ctx.stroke();
    // 最高 / 最低点
    const hiI = g.s.indexOf(Math.max.apply(null, g.s)), loI = g.s.indexOf(Math.min.apply(null, g.s));
    [[hiI, '#f2542d', -8], [loI, '#2f7be8', 16]].forEach(([i, c, dy]) => {
      ctx.fillStyle = c;
      ctx.beginPath(); ctx.arc(g.x(i), g.y(g.s[i]), 3.5, 0, Math.PI * 2); ctx.fill();
      ctx.fillText(g.s[i].toFixed(2), g.x(i) - 12, g.y(g.s[i]) + dy);
    });
    // 选中的那一轮
    const p = this.data.picked;
    if (p) {
      const xx = g.x(p.i), yy = g.y(g.s[p.i]);
      ctx.strokeStyle = 'rgba(29,35,41,0.35)';
      ctx.lineWidth = 1;
      ctx.beginPath(); ctx.moveTo(xx, g.T); ctx.lineTo(xx, h - g.B); ctx.stroke();
      ctx.fillStyle = '#ffffff'; ctx.strokeStyle = '#13a874'; ctx.lineWidth = 2;
      ctx.beginPath(); ctx.arc(xx, yy, 5, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
    }
  },

  onTouch(e) {
    if (!this.cv) return;
    const t = (e.touches && e.touches[0]) || (e.changedTouches && e.changedTouches[0]);
    if (!t) return;
    const g = this.geom();
    const xx = t.x !== undefined ? t.x : t.clientX - (e.currentTarget.offsetLeft || 0);
    let i = Math.round(((xx - g.L) / (this.cv.w - g.L - g.R)) * (g.s.length - 1));
    i = Math.max(0, Math.min(g.s.length - 1, i));
    this.setData({ picked: { i, d: TREND.dates[i], v: g.s[i].toFixed(2) } });
    this.draw();
  },
  pickOil(e) {
    const oil = e.currentTarget.dataset.oil;
    this.setData({ oil, picked: null });
    store.setOilPref(oil);
    this.summary();
  },
  remind() {
    wx.requestSubscribeMessage({
      tmplIds: ['YjOilPriceAdjust01'],
      complete: () => { wx.setStorageSync('fuel_price_remind', 1); wx.showToast({ title: '调价前一天提醒你', icon: 'success' }); }
    });
  },
  onShareAppMessage() { return { title: `浙江 ${this.data.oil} 今日 ¥${this.data.cur}/升，下轮调价 ${TREND.next}`, path: '/pages/trend/trend' }; }
});
