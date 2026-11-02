// 行程列表：进行中的那一单置顶；待支付 / 已完成 / 已取消，统计数字来自 store.stats()
const store = require('../../utils/store.js');
const order = require('../../utils/order.js');
const TABS = [{ key: 'all', name: '全部' }, { key: 'done', name: '已完成' }, { key: 'cancel', name: '已取消' }];

function decorate(t) {
  const unpaid = t.status === 'unpaid';
  let statusText = '已完成';
  if (unpaid) statusText = t.cancelled ? '取消费待支付' : '待支付';
  else if (t.status === 'cancel') statusText = '已取消';
  let foot = '';
  if (t.status === 'done') foot = '实付 ¥' + t.pay;
  else if (unpaid) foot = '待付 ¥' + store.dueOf(t);
  else if (t.cancelFee) foot = '取消费 ¥' + t.cancelFee;
  return {
    id: t.id, type: t.type, date: t.date, status: t.status, statusText, fromName: t.from.name, toName: t.to.name,
    km: t.km, min: t.min, foot, unpaid, needRate: t.status === 'done' && !t.rating, passenger: t.passenger ? t.passenger.name : ''
  };
}

Page({
  data: { tabs: TABS, tab: 'all', list: [], stat: {}, ongoing: null },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    this.load();
  },
  load() {
    const all = store.getTrips().map(decorate);
    const tab = this.data.tab;
    const o = order.get();
    this.setData({
      list: tab === 'all' ? all : all.filter(t => t.status === tab),
      stat: store.stats(),
      ongoing: o ? order.banner(o) : null
    });
  },
  onTab(e) {
    this.setData({ tab: e.currentTarget.dataset.key });
    this.load();
  },
  onOpen(e) {
    const id = String(e.currentTarget.dataset.id);
    const t = store.findTrip(id);
    if (t && t.status === 'unpaid') wx.navigateTo({ url: '/pages/pay/pay?id=' + id });
    else wx.navigateTo({ url: '/pages/trip-detail/trip-detail?id=' + id });
  },
  onRate(e) {
    wx.navigateTo({ url: '/pages/trip-detail/trip-detail?id=' + e.currentTarget.dataset.id + '&rate=1' });
  },
  onOngoing() {
    wx.navigateTo({ url: '/pages/calling/calling' });
  },
  onGoCall() {
    wx.switchTab({ url: '/pages/index/index' });
  }
});
