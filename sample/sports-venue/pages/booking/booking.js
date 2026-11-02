// 场地预订：7 天日期条 + 场地 × 时段网格（三态：可订 / 已订 / 已选），连续时段按场地合并计价
const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');
const S = require('../../utils/slots.js');
const store = require('../../utils/store.js');

const CELL_W = 72;
const MAX = 8;

Page({
  data: { v: null, days: [], day: 0, courts: [], rows: [], gridW: 0, sl: 0, fixHead: false, segs: [], total: 0, count: 0, peakN: 0 },
  onLoad(q) {
    const v = D.getVenue(q.id) || D.VENUES[0];
    this.v = v;
    const base = new Date(); base.setHours(0, 0, 0, 0);
    const days = [];
    for (let i = 0; i < 7; i++) {
      const d = new Date(base.getTime() + i * 86400000);
      days.push({ key: F.dateKey(d), top: i === 0 ? '今天' : (i === 1 ? '明天' : F.WEEK[d.getDay()]), md: (d.getMonth() + 1) + '/' + d.getDate(), weekend: d.getDay() === 0 || d.getDay() === 6 });
    }
    this.setData({ v, days, gridW: v.courts * CELL_W });
    wx.setNavigationBarTitle({ title: v.name });
    this.loadGrid();
  },
  onShow() { if (this.loaded) this.loadGrid(true); this.loaded = true; },
  loadGrid(keep) {
    const g = S.grid(this.v, this.data.days[this.data.day].key);
    // 返回本页时保留仍然可订的已选格子
    const was = keep ? this.picked || {} : {};
    this.picked = {};
    g.rows.forEach((r) => r.cells.forEach((c) => { if (was[c.key] && c.state === 'free') { c.on = true; this.picked[c.key] = c; } }));
    this.setData({ courts: g.courts, rows: g.rows });
    this.recalc();
  },
  pickDay(e) {
    const i = Number(e.currentTarget.dataset.i);
    if (i === this.data.day) return;
    const had = Object.keys(this.picked || {}).length;
    this.setData({ day: i });
    this.loadGrid(false);
    if (had) wx.showToast({ title: '已切换日期，请重新选择时段', icon: 'none' });
  },
  onReady() {
    wx.createSelectorQuery().select('#ghead').boundingClientRect((r) => { if (r) this.headTop = r.top; }).exec();
  },
  // 表头滚出视口后显示 fixed 的那份（冻结首行）
  onPageScroll(e) {
    const fix = this.headTop !== undefined && e.scrollTop >= this.headTop;
    if (fix !== this.data.fixHead) this.setData({ fixHead: fix });
  },
  // 表体横滑时把 scrollLeft 推给表头（两份表头都跟）；时段列在横向滚动区外 = 冻结首列
  onHScroll(e) { this.setData({ sl: Math.round(e.detail.scrollLeft) }); },
  tapCell(e) {
    const ri = Number(e.currentTarget.dataset.r), ci = Number(e.currentTarget.dataset.c);
    const c = this.data.rows[ri].cells[ci];
    if (c.state === 'sold' || c.state === 'mine') { wx.showToast({ title: c.state === 'mine' ? '你已预订该时段' : '该时段已被预订', icon: 'none' }); return; }
    if (c.state === 'past') { wx.showToast({ title: '该时段已过', icon: 'none' }); return; }
    const on = !c.on;
    if (on && Object.keys(this.picked).length >= MAX) { wx.showToast({ title: '单次最多预订 ' + MAX + ' 小时', icon: 'none' }); return; }
    if (on) this.picked[c.key] = c; else delete this.picked[c.key];
    const d = {}; d['rows[' + ri + '].cells[' + ci + '].on'] = on;
    this.setData(d);
    this.recalc();
  },
  recalc() {
    const list = Object.keys(this.picked).map((k) => this.picked[k]);
    const segs = S.merge(this.v, list);
    this.setData({ segs, total: segs.reduce((a, s) => a + s.amount, 0), count: list.length, peakN: list.filter((c) => c.peak).length });
  },
  clear() { this.loadGrid(false); },
  submit() {
    if (!this.data.segs.length) { wx.showToast({ title: '请先选择场地时段', icon: 'none' }); return; }
    store.setDraft({ venueId: this.v.id, date: this.data.days[this.data.day].key, segs: this.data.segs, total: this.data.total });
    wx.navigateTo({ url: '/pages/confirm/confirm' });
  }
});
