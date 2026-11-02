const store = require('../../utils/store.js');
const ORD = require('../../utils/order.js');
const TABS = [
  { key: 'all', text: '全部' }, { key: 'paid', text: '待使用' }, { key: 'done', text: '已完成' }, { key: 'cancelled', text: '已取消' }
];
// 「已完成」= 已入场 + 已结束
function match(tab, st) {
  if (tab === 'all') return true;
  if (tab === 'done') return st === 'used' || st === 'ended';
  return st === tab;
}
Page({
  data: { tabs: TABS, tab: 'all', list: [], counts: {}, loading: true },
  onLoad(q) { if (q.tab) this.setData({ tab: q.tab }); },
  onShow() { this.load(); },
  onPullDownRefresh() { this.load(); wx.stopPullDownRefresh(); },
  load() {
    const all = store.orders().map(ORD.view);
    const counts = {};
    TABS.forEach((t) => { counts[t.key] = all.filter((o) => match(t.key, o.st)).length; });
    // 待使用的按开场时间排在最前，其余按下单时间倒序
    const list = all.filter((o) => match(this.data.tab, o.st)).sort((a, b) => {
      if (a.st === 'paid' && b.st !== 'paid') return -1;
      if (b.st === 'paid' && a.st !== 'paid') return 1;
      return b.createdAt - a.createdAt;
    });
    this.setData({ list, counts, loading: false });
  },
  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.load(); },
  open(e) { wx.navigateTo({ url: '/pages/order/order?id=' + e.currentTarget.dataset.id }); },
  pass(e) { wx.navigateTo({ url: '/pages/pass/pass?id=' + e.currentTarget.dataset.id }); },
  again(e) { wx.navigateTo({ url: '/pages/booking/booking?id=' + e.currentTarget.dataset.v }); },
  toHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
