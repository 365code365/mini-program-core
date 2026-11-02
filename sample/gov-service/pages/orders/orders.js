const store = require('../../utils/store');

const TABS = [
  { k: 'all', t: '全部' }, { k: 'doing', t: '办理中' }, { k: 'done', t: '已办结' }, { k: 'appoint', t: '我的预约' }
];

Page({
  data: { tabs: TABS, tab: 'all', list: [], appoints: [], loading: true },
  onShow() {
    store.syncTab(this, 1);
    const app = getApp();
    if (app.globalData.orderTab) { this.setData({ tab: app.globalData.orderTab }); app.globalData.orderTab = ''; }
    this.refresh();
  },
  refresh() {
    const tab = this.data.tab;
    const today = store.fmtDate(Date.now());
    const list = store.getOrders()
      .filter(o => tab === 'all' || (tab === 'doing' ? ['submitted', 'reviewing', 'supplement'].indexOf(o.status) >= 0 : (tab === 'done' ? ['done', 'withdrawn'].indexOf(o.status) >= 0 : false)))
      .map(o => Object.assign({}, o, { st: store.STATUS[o.status], time: store.fmtTime(o.createdAt), last: o.timeline[o.timeline.length - 1].title }));
    const appoints = store.getAppoints().map(a => {
      const status = a.status === 'booked' && a.date < today ? 'expired' : a.status;
      return Object.assign({}, a, { status, stText: { booked: '待到场', cancelled: '已取消', expired: '已过期' }[status] });
    });
    this.setData({ list, appoints, loading: false });
  },
  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.k }, () => this.refresh()); },
  open(e) { wx.navigateTo({ url: '/pages/progress/progress?id=' + e.currentTarget.dataset.id }); },
  cancelAppoint(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '取消预约', content: '确定取消这次预约吗？', confirmText: '确定取消',
      success: (r) => {
        if (!r.confirm) return;
        const l = store.getAppoints();
        const a = l.find(x => x.id === id);
        if (a) a.status = 'cancelled';
        store.saveAppoints(l);
        this.refresh();
      }
    });
  },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  goAppoint() { wx.navigateTo({ url: '/pages/appoint/appoint' }); }
});
