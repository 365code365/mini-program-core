// 展览日历：月历（flex-wrap 七列；引擎的 display:grid 不排版，见 engine-issues）+ 选中日的特展与社教活动。
// 活动报名按「活动 id@日期」存 storage，退出重进还在；可预约的日期直接带 date 跳预约页
const { expos, WEEKLY, ymd } = require('../../utils/data.js');
const U = require('../../utils/util.js');

function leftOf(ev, date) {
  let h = 0;
  for (const c of ev.id + date) h = (h * 33 + c.charCodeAt(0)) % 1000;
  return Math.max(0, ev.quota - 4 - (h % ev.quota));
}

Page({
  data: { weeks: ['日', '一', '二', '三', '四', '五', '六'], y: 0, m: 0, cells: [], sel: '', info: null, canPrev: false, canNext: true },
  onLoad() {
    const t = new Date();
    this.base = { y: t.getFullYear(), m: t.getMonth() };
    this.today = ymd(t);
    this.setData({ y: t.getFullYear(), m: t.getMonth(), sel: this.today });
    this.build();
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.pick(this.data.sel);
  },
  build() {
    const { y, m } = this.data;
    const first = new Date(y, m, 1), days = new Date(y, m + 1, 0).getDate();
    const ex = expos(), cells = [];
    for (let i = 0; i < first.getDay(); i++) cells.push({ k: 'b' + i, blank: true });
    for (let d = 1; d <= days; d++) {
      const dt = new Date(y, m, d), date = ymd(dt);
      const closed = U.isClosed(dt);
      const start = ex.some(x => x.start === date);
      cells.push({ k: date, d, date, closed, past: date < this.today, today: date === this.today,
        ev: !closed && !!WEEKLY[dt.getDay()], start });
    }
    const off = (y - this.base.y) * 12 + m - this.base.m;
    this.setData({ cells, canPrev: off > -1, canNext: off < 3 });
  },
  shift(e) {
    const dir = +e.currentTarget.dataset.d;
    if ((dir < 0 && !this.data.canPrev) || (dir > 0 && !this.data.canNext)) return;
    const dt = new Date(this.data.y, this.data.m + dir, 1);
    this.setData({ y: dt.getFullYear(), m: dt.getMonth() });
    this.build();
  },
  onDay(e) { if (e.currentTarget.dataset.date) this.pick(e.currentTarget.dataset.date); },
  pick(date) {
    const dt = new Date(date.replace(/-/g, '/'));
    const closed = U.isClosed(dt);
    const signed = U.load('events', []);
    const on = expos().filter(x => x.start <= date && date <= x.end);
    const soon = expos().filter(x => x.start > date).slice(0, 2);
    const events = closed ? [] : (WEEKLY[dt.getDay()] || []).map(ev => Object.assign({}, ev, {
      left: leftOf(ev, date), signed: signed.indexOf(ev.id + '@' + date) >= 0 }));
    const bookable = U.bookDays().some(d => d.date === date);
    this.setData({ sel: date, info: { closed, expos: on, soon, events, bookable, past: date < this.today,
      title: (dt.getMonth() + 1) + ' 月 ' + dt.getDate() + ' 日 ' + U.WEEK[dt.getDay()] } });
  },
  signup(e) {
    const { id, title, time } = e.currentTarget.dataset;
    const date = this.data.sel, key = id + '@' + date;
    if (this.data.info.past) { wx.showToast({ title: '活动已结束', icon: 'none' }); return; }
    const list = U.load('events', []);
    if (list.indexOf(key) >= 0) {
      wx.showModal({
        title: '取消报名', content: '确定取消「' + title + '」的报名吗？', confirmText: '取消报名', cancelText: '再想想', confirmColor: '#c0392b',
        success: r => { if (r.confirm) { U.save('events', list.filter(k => k !== key)); this.pick(date); wx.showToast({ title: '已取消', icon: 'none' }); } }
      });
      return;
    }
    const ev = this.data.info.events.find(x => x.id === id);
    if (!ev.left) { wx.showToast({ title: '名额已满', icon: 'none' }); return; }
    list.push(key);
    U.save('events', list);
    this.pick(date);
    wx.showModal({ title: '报名成功', content: date + ' ' + time + ' 「' + title + '」，请提前 10 分钟到 ' + ev.place + ' 签到。', showCancel: false, confirmText: '知道了' });
  },
  openExpo(e) { wx.navigateTo({ url: '/pages/expo/expo?id=' + e.currentTarget.dataset.id }); },
  book() { wx.navigateTo({ url: '/pages/booking/booking?date=' + this.data.sel }); }
});
