const T = require('../../utils/store.js');

Page({
  data: { trips: 0, days: 0, cities: 0, spots: 0, budgetText: '0', next: '' },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.refresh();
  },
  refresh() {
    const list = T.all();
    const now = T.toStr(T.today());
    let days = 0, spots = 0, budget = 0;
    const cities = {};
    list.forEach(t => { days += t.plan.length; spots += t.plan.reduce((n, d) => n + d.length, 0); budget += T.budgetSum(t.budget); cities[t.city] = 1; });
    const up = list.filter(t => t.start >= now).sort((a, b) => (a.start < b.start ? -1 : 1))[0];
    this.setData({
      trips: list.length, days, spots, cities: Object.keys(cities).length, budgetText: T.money(budget),
      next: up ? up.title + ' · ' + T.diff(now, up.start) + ' 天后' : '暂无'
    });
  },
  clearAll() {
    wx.showModal({
      title: '清空全部行程', content: '所有行程、预算和行李清单都会被删除，且无法恢复', confirmText: '清空', confirmColor: '#f5222d',
      success: r => { if (r.confirm) { T.save([]); this.refresh(); wx.showToast({ title: '已清空', icon: 'none' }); } }
    });
  },
  restore() {
    wx.removeStorageSync('tv_trips');
    this.refresh();
    wx.showToast({ title: '已恢复示例行程', icon: 'none' });
  },
  goTrips() { wx.switchTab({ url: '/pages/trips/trips' }); }
});
