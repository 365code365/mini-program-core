const store = require('../../utils/store.js');
const flows = require('../../utils/flows.js');
// 我的订单：?type=goods（商城）| service（服务预约），?tab= 状态
const TABS = {
  goods: [{ key: 'all', name: '全部' }, { key: 'unpaid', name: '待付款' }, { key: 'paid', name: '待发货' }, { key: 'shipped', name: '待收货' }, { key: 'rate', name: '待评价' }],
  service: [{ key: 'all', name: '全部' }, { key: 'booked', name: '待服务' }, { key: 'done', name: '已完成' }, { key: 'cancel', name: '已取消' }]
};
function match(tab, o) {
  if (tab === 'all') return true;
  if (tab === 'rate') return o.status === 'done' && !o.rating;
  return o.status === tab;
}
Page({
  data: { type: 'goods', tabs: TABS.goods, tab: 'all', list: [] },
  onLoad(q) {
    const type = q.type === 'service' ? 'service' : 'goods';
    this.setData({ type, tabs: TABS[type], tab: TABS[type].some(t => t.key === q.tab) ? q.tab : 'all' });
  },
  onShow() { this.refresh(); },
  refresh() {
    store.tick();
    const d = this.data;
    const list = store.getOrders().filter(o => o.kind === d.type && match(d.tab, o)).map(o => {
      const st = store.stateOf(o);
      const goods = o.kind === 'goods';
      return {
        id: o.id, status: o.status, st, pay: o.pay, time: store.dateTime(o.createdAt),
        imgs: goods ? o.items.slice(0, 3).map(x => x.img) : [],
        summary: goods ? o.items.map(x => x.name + ' ×' + x.qty).join('、') : '',
        // 左下角：取消了写退款去向，否则写件数 / 支付方式（空串的话引擎会把右边的金额挤到左边）
        footText: o.status === 'cancel' ? st.tip : (goods ? '共 ' + o.count + ' 件' : (o.cardUsed ? (o.pay > 0 ? '洗护卡 + 微信支付' : '洗护卡抵扣') : '微信支付')),
        title: goods ? '' : o.petName + ' · ' + o.svcNames,
        sub: goods ? '' : o.date + ' ' + o.week + ' ' + o.time + ' · ' + o.storeName
      };
    });
    this.setData({ list });
  },
  setType(e) {
    const type = e.currentTarget.dataset.t;
    this.setData({ type, tabs: TABS[type], tab: 'all' });
    this.refresh();
  },
  setTab(e) {
    this.setData({ tab: e.currentTarget.dataset.k });
    this.refresh();
  },
  find(e) { return store.findOrder(e.currentTarget.dataset.id); },
  goDetail(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  onPay(e) { flows.pay(this.find(e)).then(() => this.refresh()); },
  onCancel(e) { flows.cancel(this.find(e)).then(() => this.refresh()); },
  onConfirm(e) { flows.confirm(this.find(e)).then(() => this.refresh()); },
  onRate(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?rate=1&id=' + e.currentTarget.dataset.id }); },
  onAgain(e) { flows.again(this.find(e)); },
  goShop() {
    if (this.data.type === 'goods') wx.switchTab({ url: '/pages/mall/mall' });
    else wx.navigateTo({ url: '/pages/booking/booking' });
  }
});
