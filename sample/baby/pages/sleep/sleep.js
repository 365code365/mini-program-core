const store = require('../../utils/store.js');
const T = require('../../utils/time.js');

const WEEK = ['日', '一', '二', '三', '四', '五', '六'];

Page({
  data: {
    running: false, elapsed: '00:00', startText: '',
    date: '', today: '', start: '21:00', end: '06:30', cross: true, err: '',
    bars: [], avg: '', list: []
  },

  onLoad() {
    const now = Date.now();
    // 补录多半是补昨晚的夜觉：默认昨天 21:00 → 次日 06:30
    this.setData({ date: T.ymd(now - T.DAY), today: T.ymd(now) });
  },
  onShow() {
    this.tick();
    this.load();
    this._iv = setInterval(() => this.tick(), 1000);
  },
  onHide() { clearInterval(this._iv); },
  onUnload() { clearInterval(this._iv); },

  tick() {
    const t = store.get(store.K.sleepTimer, null);
    if (!t) { if (this.data.running) this.setData({ running: false }); return; }
    this.setData({ running: true, elapsed: T.clock((Date.now() - t.startedAt) / 1000), startText: T.hm(t.startedAt) });
  },

  load() {
    const all = store.list(store.K.sleeps);
    const today0 = T.startOfDay(Date.now());
    const bars = [];
    let max = 1, sum = 0;
    for (let i = 6; i >= 0; i--) {
      const d0 = today0 - i * T.DAY, d1 = d0 + T.DAY;
      // 落在这一天里的睡眠分钟数（跨夜的按自然日拆开）
      const min = all.reduce((s, r) => s + Math.max(0, Math.min(r.end, d1) - Math.max(r.start, d0)) / 60000, 0);
      max = Math.max(max, min); sum += min;
      bars.push({ key: 'd' + i, label: i === 0 ? '今天' : '周' + WEEK[new Date(d0).getDay()], min, hours: (min / 60).toFixed(1) });
    }
    bars.forEach((b) => { b.h = Math.max(6, Math.round(b.min / max * 200)); });
    const list = all.slice().sort((a, b) => b.end - a.end).slice(0, 10).map((r) => ({
      id: r.id, day: T.isToday(r.end) ? '今天' : T.ymd(r.end).slice(5),
      range: T.hm(r.start) + ' - ' + T.hm(r.end), len: T.duration((r.end - r.start) / 60000),
      night: new Date(r.start).getHours() >= 19 || new Date(r.start).getHours() < 5
    }));
    this.setData({ bars, avg: T.duration(sum / 7), list });
  },

  startSleep() {
    store.set(store.K.sleepTimer, { startedAt: Date.now() });
    wx.vibrateShort({ type: 'light' });
    wx.showToast({ title: '晚安，开始记录睡眠', icon: 'none' });
    this.tick();
  },

  wake() {
    const t = store.get(store.K.sleepTimer, null);
    if (!t) return;
    const min = (Date.now() - t.startedAt) / 60000;
    wx.removeStorageSync(store.K.sleepTimer);
    if (min < 1) {
      wx.showToast({ title: '不到 1 分钟，未保存', icon: 'none' });
    } else {
      store.add(store.K.sleeps, { start: t.startedAt, end: Date.now() });
      wx.vibrateShort({ type: 'medium' });
      wx.showToast({ title: '睡了 ' + T.duration(min), icon: 'success' });
    }
    this.tick();
    this.load();
  },

  cancelSleep() {
    wx.removeStorageSync(store.K.sleepTimer);
    this.tick();
  },

  onDate(e) { this.setData({ date: e.detail.value, err: '' }); this.checkCross(); },
  onStart(e) { this.setData({ start: e.detail.value, err: '' }); this.checkCross(); },
  onEnd(e) { this.setData({ end: e.detail.value, err: '' }); this.checkCross(); },
  checkCross() { this.setData({ cross: this.data.end <= this.data.start }); },

  saveManual() {
    const s = T.parse(this.data.date, this.data.start);
    let e = T.parse(this.data.date, this.data.end);
    if (e <= s) e += T.DAY;   // 结束早于开始：睡过了午夜
    const min = (e - s) / 60000;
    if (e > Date.now()) return this.setData({ err: '醒来时间不能晚于现在' });
    if (min > 16 * 60) return this.setData({ err: '单次睡眠超过 16 小时，请检查时间' });
    const overlap = store.list(store.K.sleeps).some((r) => r.start < e && r.end > s);
    if (overlap) return this.setData({ err: '和已有的睡眠记录时间重叠了' });
    store.add(store.K.sleeps, { start: s, end: e });
    wx.showToast({ title: '已补录 ' + T.duration(min), icon: 'success' });
    this.setData({ err: '' });
    this.load();
  },

  onItem(e) {
    const id = e.currentTarget.dataset.id;
    wx.showActionSheet({
      itemList: ['删除这条记录'],
      itemColor: '#f0506e',
      success: () => { store.remove(store.K.sleeps, id); this.load(); }
    });
  }
});
