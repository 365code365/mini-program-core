const T = require('../../utils/store.js');
const G = require('../../utils/guide.js');

Page({
  data: { trips: 0, days: 0, cities: 0, spots: 0, budgetText: '0', spentText: '0', next: '', wishText: '' },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.refresh();
  },
  refresh() {
    const list = T.all();
    const now = T.toStr(T.today());
    let days = 0, spots = 0, budget = 0, spent = 0;
    const cities = {};
    list.forEach(t => { days += t.plan.length; spots += t.plan.reduce((n, d) => n + d.length, 0); budget += T.budgetSum(t.budget); spent += T.expenseSum(t); cities[t.city] = 1; });
    const up = list.filter(t => t.start >= now).sort((a, b) => (a.start < b.start ? -1 : 1))[0];
    this.setData({
      trips: list.length, days, spots, cities: Object.keys(cities).length, budgetText: T.money(budget), spentText: T.money(spent),
      wishText: G.wishes().length ? G.wishes().slice(0, 3).join('、') + (G.wishes().length > 3 ? ' 等 ' + G.wishes().length + ' 个' : '') : '去灵感页标记',
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
  goTrips() { wx.switchTab({ url: '/pages/trips/trips' }); },
  goExplore() { wx.switchTab({ url: '/pages/explore/explore' }); },
  // 灵感页是 tab 页，switchTab 不能带参数：用 globalData 告诉它切到「想去」
  goWish() { getApp().globalData = Object.assign(getApp().globalData || {}, { exploreTag: '想去' }); wx.switchTab({ url: '/pages/explore/explore' }); },
  goImport() { wx.navigateTo({ url: '/pages/import/import' }); }
});
