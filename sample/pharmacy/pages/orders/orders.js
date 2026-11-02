// 我的订单：按状态筛选，点卡片展开订单详情（地址、处方号、金额明细、时间线）；
// 状态流转：待发货 →（出库约 1 分钟后）配送中 → 确认收货 → 已完成；待发货可取消；已完成可再次购买
const S = require('../../utils/store.js');

const TABS = ['全部', '待发货', '配送中', '已完成', '已取消'];
const STC = { '待发货': 'w', '配送中': 's', '已完成': 'd', '已取消': 'c' };
function fmt(t) {
  const d = new Date(t), p = n => ('0' + n).slice(-2);
  return (d.getMonth() + 1) + '-' + p(d.getDate()) + ' ' + p(d.getHours()) + ':' + p(d.getMinutes());
}

Page({
  data: { tabs: TABS, tab: 0, list: [], open: '' },
  onLoad(q) { const i = TABS.indexOf(q.status || ''); if (i > 0) this.setData({ tab: i }); },
  onShow() { this.load(); },
  onPullDownRefresh() { this.load(); wx.stopPullDownRefresh(); },
  load() {
    const all = S.get('orders', []);
    if (S.advanceOrders(all)) S.set('orders', all);
    const name = TABS[this.data.tab];
    const list = all.filter(o => name === '全部' || o.status === name).map(o => Object.assign({}, o, {
      stc: STC[o.status], items: o.items.map(i => Object.assign({}, i, { priceText: S.money(i.price) })), time: fmt(o.createdAt), count: o.items.reduce((a, i) => a + i.qty, 0),
      line: [{ t: '下单并支付', at: fmt(o.createdAt) }]
        .concat(o.shipAt ? [{ t: '药房出库，骑手配送中', at: fmt(o.shipAt) }] : [])
        .concat(o.doneAt ? [{ t: '已签收', at: fmt(o.doneAt) }] : [])
        .concat(o.cancelAt ? [{ t: '订单已取消，退款原路返回', at: fmt(o.cancelAt) }] : [])
    }));
    this.setData({ list });
  },
  onTab(e) { this.setData({ tab: +e.currentTarget.dataset.i, open: '' }); this.load(); },
  toggle(e) { const id = e.currentTarget.dataset.id; this.setData({ open: this.data.open === id ? '' : id }); },
  update(id, fn) {
    const all = S.get('orders', []);
    fn(all.find(o => o.id === id));
    S.set('orders', all);
    this.load();
  },
  cancel(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '取消订单', content: '药房尚未出库，取消后款项将在 1~3 个工作日原路退回。', confirmText: '取消订单', cancelText: '不取消', confirmColor: '#f04a3e',
      success: r => { if (r.confirm) { this.update(id, o => { o.status = '已取消'; o.cancelAt = Date.now(); }); wx.showToast({ title: '已取消', icon: 'none' }); } }
    });
  },
  urge() { wx.showToast({ title: '已提醒药房尽快出库', icon: 'none' }); },
  receive(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '确认收货', content: '请确认药品包装完好、数量无误后再确认收货。', confirmText: '确认收货', confirmColor: '#12a37f',
      success: r => { if (r.confirm) { this.update(id, o => { o.status = '已完成'; o.doneAt = Date.now(); }); wx.showToast({ title: '已确认收货', icon: 'success' }); } }
    });
  },
  rebuy(e) {
    const o = this.data.list.find(x => x.id === e.currentTarget.dataset.id);
    o.items.forEach(i => S.addCart(i.id, i.si || 0, i.qty));
    wx.showToast({ title: '已加入购物车', icon: 'none' });
    setTimeout(() => wx.switchTab({ url: '/pages/cart/cart' }), 500);
  },
  remind(e) { wx.navigateTo({ url: '/pages/remind-edit/remind-edit?drug=' + encodeURIComponent(e.currentTarget.dataset.name) }); },
  goShop() { wx.switchTab({ url: '/pages/index/index' }); }
});
