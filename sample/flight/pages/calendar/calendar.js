const D = require('../../utils/date');
const F = require('../../utils/flights');

// 月历：单程点一天就回；往返先点去程再点返程（区间高亮）。每格是该航线当天的最低经济舱价，月内低价标绿。
// 结果经 EventChannel 的 pickDate 回给首页
Page({
  data: { mode: 'single', dep: '', ret: '', step: 0, months: [], hint: '', week: ['日', '一', '二', '三', '四', '五', '六'] },
  onLoad(q) {
    this.route2 = { from: q.from || 'HGH', to: q.to || 'BJS' };
    this.setData({ mode: q.mode || 'single', dep: q.dep || D.addDays(D.today(), 1), ret: q.mode === 'range' ? (q.ret || '') : '' });
    this.channel = this.getOpenerEventChannel ? this.getOpenerEventChannel() : null;
    if (this.channel && this.channel.on) {
      this.channel.on('init', (d) => { this.setData({ mode: d.mode, dep: d.dep, ret: d.mode === 'range' ? d.ret : '' }); this.build(); });
    }
    wx.setNavigationBarTitle({ title: q.mode === 'range' ? '选择往返日期' : '选择出发日期' });
    this.build();
  },
  build() {
    const { mode, dep, ret, step } = this.data;
    const today = D.today(), t = D.parse(today);
    const months = [];
    for (let m = 0; m < 4; m++) {
      const first = new Date(t.getFullYear(), t.getMonth() + m, 1);
      const days = new Date(first.getFullYear(), first.getMonth() + 1, 0).getDate();
      const cells = [];
      for (let i = 0; i < first.getDay(); i++) cells.push({ key: 'e' + i, empty: true });
      let min = Infinity;
      for (let d = 1; d <= days; d++) {
        const ds = D.fmt(new Date(first.getFullYear(), first.getMonth(), d));
        const past = D.diffDays(today, ds) < 0;
        const price = past ? 0 : F.lowPrice(this.route2.from, this.route2.to, ds);
        if (price && price < min) min = price;
        let sel = '';
        if (ds === dep) sel = mode === 'range' && ret ? 'start' : 'single';
        else if (mode === 'range' && ret && ds === ret) sel = 'end';
        else if (mode === 'range' && ret && ds > dep && ds < ret) sel = 'in';
        cells.push({ key: ds, date: ds, day: d, price, past, sel, today: ds === today, tip: ds === today ? '今天' : '' });
      }
      cells.forEach(c => { if (c.price && c.price <= min * 1.12) c.cheap = true; });
      months.push({ title: first.getFullYear() + '年' + (first.getMonth() + 1) + '月', cells, min });
    }
    const hint = mode === 'single' ? '请选择出发日期' : step === 1 ? '请选择返程日期' : '请选择去程日期';
    this.setData({ months, hint });
  },
  tap(e) {
    const ds = e.currentTarget.dataset.d;
    if (!ds) return;
    const { mode, dep, step } = this.data;
    if (mode === 'single') return this.done(ds, '');
    if (step === 0 || ds <= dep) {
      this.setData({ dep: ds, ret: '', step: 1 });
      this.build();
      return;
    }
    this.setData({ ret: ds, step: 0 });
    this.build();
    setTimeout(() => this.done(this.data.dep, ds), 250);
  },
  done(dep, ret) {
    if (this.channel && this.channel.emit) this.channel.emit('pickDate', { dep, ret });
    wx.navigateBack();
  }
});
