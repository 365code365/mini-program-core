// 我买到的：按状态筛选，点进订单详情
const store = require('../../utils/store.js');
const TABS = [{ k: 'all', n: '全部' }, { k: 'paid', n: '待发货' }, { k: 'shipped', n: '待收货' }, { k: 'done', n: '已完成' }, { k: 'closed', n: '退款' }];
Page({
  data: { tabs: TABS, tab: 'all', list: [] },
  onLoad(q) { if (q.tab) this.setData({ tab: q.tab }); },
  onShow() { this.refresh(); },
  refresh() {
    const t = this.data.tab;
    const list = store.orders().filter(o => t === 'all' || o.status === t)
      .map(o => Object.assign({}, o, { statusText: store.STATUS[o.status], timeText: store.timeText(o.created) }));
    this.setData({ list });
  },
  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.refresh(); },
  open(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
