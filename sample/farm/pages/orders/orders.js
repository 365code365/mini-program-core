const store = require('../../utils/store.js');

const TABS = [
  { k: 'all', t: '全部' }, { k: 'unpaid', t: '待付款' }, { k: 'grouping', t: '待成团' }, { k: 'paid', t: '待发货' },
  { k: 'shipped', t: '待收货' }, { k: 'done', t: '已完成' }, { k: 'after', t: '售后' }
];
const ACT = { unpaid: ['pay', '去支付'], grouping: ['group', '邀请参团'], paid: ['remind', '提醒发货'], shipped: ['receive', '确认收货'], done: ['again', '再来一单'] };
const MODE = { single: '单买', cart: '购物车', group: '开团', join: '参团' };

Page({
  data: { tabs: TABS, tab: 'all', list: [], loading: true },
  onLoad(q) { this.setData({ tab: q.tab || 'all' }); },
  onShow() {
    // 拼团单先结算一次（满员 / 超时）再出列表
    store.orders().forEach((o) => { if (o.groupId && o.status === 'grouping') store.settleGroup(o.groupId); });
    this.setData({ loading: true });
    setTimeout(() => this.load(), 200);
  },
  load() {
    const all = store.orders();
    const hit = (o, k) => k === 'all' || (k === 'after' ? o.status === 'refunding' || o.status === 'refunded' : o.status === k);
    const tabs = TABS.map((t) => Object.assign({}, t, { n: t.k === 'all' || t.k === 'done' ? 0 : all.filter((o) => hit(o, t.k)).length }));
    const list = all.filter((o) => hit(o, this.data.tab)).map((o) => ({
      id: o.id, status: o.status, statusText: store.STATUS[o.status], items: o.items, total: o.total, timeText: store.fmtTime(o.createdAt),
      modeText: MODE[o.mode] || '', act: (ACT[o.status] || [])[0], actText: (ACT[o.status] || [])[1]
    }));
    this.setData({ tabs, list, loading: false });
  },
  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.load(); },
  open(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  doAct(e) { wx.navigateTo({ url: `/pages/order-detail/order-detail?id=${e.currentTarget.dataset.id}&act=${e.currentTarget.dataset.act}` }); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
