const app = getApp();
Page({
  data: { tabs: [{ key: 'all', name: '全部' }, { key: 'done', name: '已完成' }, { key: 'cancel', name: '已取消' }], tab: 'all', list: [], stat: {} },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 }); this.load(); },
  load() {
    const all = app.getTrips().map(t => Object.assign({}, t, { total: t.status === 'done' ? app.fees(t).total.toFixed(1) : '0' }));
    const done = all.filter(t => t.status === 'done');
    const km = done.reduce((s, t) => s + t.km, 0);
    const saved = done.reduce((s, t) => s + app.fees(t).coupon, 0);
    this.all = all;
    this.setData({
      list: this.data.tab === 'all' ? all : all.filter(t => t.status === this.data.tab),
      stat: { count: all.length, km: km.toFixed(1), saved }
    });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.key }); this.load(); },
  onOpen(e) { wx.navigateTo({ url: '/pages/trip-detail/trip-detail?id=' + e.currentTarget.dataset.id }); }
});
