const store = require('../../utils/store.js');

function monthText(m) { const [y, mm] = m.split('-'); return `${y} 年 ${+mm} 月`; }
function shift(m, k) { const [y, mm] = m.split('-').map(Number); const d = new Date(y, mm - 1 + k, 1); return `${d.getFullYear()}-${store.pad(d.getMonth() + 1)}`; }

Page({
  data: { month: '', maxMonth: '', monthText: '', stat: {}, bars: [], list: [], loading: true },
  onLoad() { const m = store.monthKey(Date.now()); this.setData({ month: m, maxMonth: m }); },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 2 });
    this.setData({ loading: true });
    setTimeout(() => this.load(), 150);
  },
  load() {
    const all = store.records();
    const m = this.data.month;
    const inM = all.filter((r) => store.monthKey(r.createdAt) === m);
    const sum = (l, f) => l.reduce((s, r) => s + parseFloat(r[f]), 0);
    const months = [-5, -4, -3, -2, -1, 0].map((k) => shift(this.data.maxMonth, k));
    const vals = months.map((mm) => Math.round(sum(all.filter((r) => store.monthKey(r.createdAt) === mm), 'pay')));
    const top = Math.max.apply(null, vals.concat(1));
    this.setData({
      monthText: monthText(m), loading: false,
      stat: { pay: store.money(sum(inM, 'pay')), liters: sum(inM, 'liters').toFixed(1), count: inM.length, saved: store.money(sum(inM, 'discount') + sum(inM, 'couponCut')) },
      bars: months.map((mm, i) => ({ m: mm, v: vals[i], h: Math.max(6, Math.round((vals[i] / top) * 160)), label: +mm.split('-')[1] + '月' })),
      list: inM.map((r) => Object.assign({}, r, { timeText: store.fmt(r.createdAt), saved: store.money(parseFloat(r.discount) + parseFloat(r.couponCut)) }))
    });
  },
  onMonth(e) { this.setData({ month: e.detail.value }); this.load(); },
  pickBar(e) { this.setData({ month: e.currentTarget.dataset.m }); this.load(); },
  invoice(e) { wx.navigateTo({ url: '/pages/invoice/invoice?id=' + e.currentTarget.dataset.id }); },
  again(e) {
    const r = store.recordById(e.currentTarget.dataset.id);
    wx.navigateTo({ url: `/pages/refuel/refuel?id=${r.stationId}&oil=${encodeURIComponent(r.oil)}` });
  },
  goRefuel() { wx.switchTab({ url: '/pages/index/index' }); }
});
