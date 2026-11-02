// 打卡日历：月历标出打卡日与有学习记录的日子，可翻月；点某天看当天学了多少
const store = require('../../utils/store.js');
const D = require('../../utils/date.js');

Page({
  data: { y: 0, m: 0, cells: [], streak: 0, total: 0, monthDays: 0, sel: null, canNext: false, checked: false },
  onLoad() {
    const t = new Date();
    this.setData({ y: t.getFullYear(), m: t.getMonth() + 1 });
  },
  onShow() { this.render(); },
  render() {
    const { y, m } = this.data;
    const set = {};
    store.checkins().forEach((d) => { set[d] = 1; });
    const logs = store.logs(), today = D.today();
    const first = new Date(y, m - 1, 1).getDay();
    const days = new Date(y, m, 0).getDate();
    const cells = [];
    for (let i = 0; i < first; i++) cells.push({ key: 'b' + i, d: '' });
    let monthDays = 0;
    for (let d = 1; d <= days; d++) {
      const ds = y + '-' + D.pad(m) + '-' + D.pad(d);
      const on = !!set[ds];
      if (on) monthDays++;
      cells.push({ key: ds, d, ds, on, today: ds === today, future: ds > today, act: !!logs[ds] });
    }
    const t = new Date();
    this.setData({
      cells, monthDays, streak: store.streak(), total: store.checkins().length, checked: store.checkedToday(),
      canNext: y < t.getFullYear() || (y === t.getFullYear() && m < t.getMonth() + 1)
    });
    this.pick(this.data.sel ? this.data.sel.ds : today);
  },
  pick(ds) {
    const l = Object.assign({ learn: 0, review: 0, quiz: 0 }, store.logs()[ds]);
    const on = store.checkins().indexOf(ds) >= 0;
    this.setData({ sel: { ds, label: ds.slice(5).replace('-', '月') + '日', learn: l.learn, review: l.review, quiz: l.quiz, on } });
  },
  onDay(e) {
    const ds = e.currentTarget.dataset.ds;
    if (!ds) return;
    if (ds > D.today()) { wx.showToast({ title: '还没到这一天', icon: 'none' }); return; }
    this.pick(ds);
  },
  onPrev() {
    let { y, m } = this.data;
    m -= 1; if (m < 1) { m = 12; y -= 1; }
    this.setData({ y, m, sel: null });
    this.render();
  },
  onNext() {
    if (!this.data.canNext) return;
    let { y, m } = this.data;
    m += 1; if (m > 12) { m = 1; y += 1; }
    this.setData({ y, m, sel: null });
    this.render();
  },
  goLearn() { wx.navigateTo({ url: '/pages/learn/learn' }); }
});
