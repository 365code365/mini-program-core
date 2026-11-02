const D = require('../../utils/data.js');
const seats = require('../../utils/seats.js');
const store = require('../../utils/store.js');

const MAX = 4;
const BASE = 20; // 缩放 100% 时一个座位格子的边长（px）
const AREA_H = 330;

Page({
  data: {
    show: null, sesText: '', rows: [], tiers: [], picked: [], total: 0,
    cell: BASE, zoom: 100, mvW: 0, mvH: 0, x: 0, y: 0, areaH: AREA_H, pinch: 100
  },
  onLoad(q) {
    const show = D.getShow(q.id) || D.SHOWS[0];
    const ses = show.sessions.find((s) => s.id === q.ses) || show.sessions[0];
    this.show = show; this.ses = ses;
    this.areaW = wx.getWindowInfo().windowWidth || 375;
    const map = seats.build(show, ses.id);
    this.cols = map.cols;
    this.setData({
      show, sesText: ses.date + ' ' + ses.week + ' ' + ses.time, rows: map.rows,
      tiers: show.tiers.map((t) => ({ name: t.name, price: t.price, color: t.color }))
    });
    this.resize(BASE, true);
  },
  // 座位图尺寸 = (行号列 + 18 列 + 过道) × 格子 + 左右留白；舞台条 + 13 排
  resize(cell, center) {
    const mvW = (this.cols + 1) * cell + 24;
    const mvH = this.data.rows.length * cell + 64;
    const d = { cell, zoom: Math.round(cell / BASE * 100), mvW, mvH };
    if (center) {
      // 比区域窄时居中摆放；比区域宽时从左边（排号那一侧）开始，向左拖看右半场
      d.x = mvW > this.areaW ? 0 : Math.round((this.areaW - mvW) / 2);
      d.y = mvH > AREA_H ? 0 : Math.round((AREA_H - mvH) / 2);
    }
    this.setData(d);
  },
  zoomIn() { if (this.data.cell < 32) this.resize(this.data.cell + 4, true); },
  zoomOut() { if (this.data.cell > 12) this.resize(this.data.cell - 4, true); },
  zoomReset() { this.resize(BASE, true); },
  // 双指缩放（真机）：movable-view 自己按 transform 缩放，这里只回显比例
  onScale(e) { this.setData({ pinch: Math.round((e.detail.scale || 1) * 100) }); },
  onMove(e) { this.pos = e.detail; },
  tapSeat(e) {
    const ri = Number(e.currentTarget.dataset.r), ci = Number(e.currentTarget.dataset.c);
    const s = this.data.rows[ri].seats[ci];
    if (!s || s.gap) return;
    if (s.sold) { wx.showToast({ title: '该座位已售出', icon: 'none' }); return; }
    const picked = this.data.picked.slice();
    const at = picked.findIndex((p) => p.id === s.id);
    if (at >= 0) picked.splice(at, 1);
    else {
      if (picked.length >= MAX) { wx.showToast({ title: '每单最多选 ' + MAX + ' 个座位', icon: 'none' }); return; }
      const t = this.show.tiers.find((x) => x.id === s.tier);
      picked.push({ id: s.id, ri, ci, label: s.label, price: s.price, tier: s.tier, tierName: t.name });
    }
    const d = { picked, total: picked.reduce((a, p) => a + p.price, 0) };
    d['rows[' + ri + '].seats[' + ci + '].on'] = at < 0;
    this.setData(d);
  },
  unpick(e) {
    const p = this.data.picked[Number(e.currentTarget.dataset.i)];
    if (p) this.tapSeat({ currentTarget: { dataset: { r: p.ri, c: p.ci } } });
  },
  confirm() {
    if (!this.data.picked.length) { wx.showToast({ title: '请先选择座位', icon: 'none' }); return; }
    const items = this.data.picked.map((p) => ({ seatId: p.id, label: p.label, tier: p.tier, tierName: p.tierName, price: p.price }));
    store.setDraft({ showId: this.show.id, sessionId: this.ses.id, items });
    wx.navigateTo({ url: '/pages/confirm/confirm' });
  }
});
