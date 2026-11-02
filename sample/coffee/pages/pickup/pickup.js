const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
function brief(o, now) {
  return {
    id: o.id,
    code: o.code,
    pay: o.pay,
    status: store.stageOf(o, now).status,
    summary: o.items.map(i => `${i.name}×${i.qty}`).join('、'),
    time: store.dateTime(o.createdAt)
  };
}
Page({
  data: {
    order: null,
    st: null,
    store: data.stores[0],
    steps: [],
    history: [],
    total: 0
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.load();
    // 制作进度按时间推进：停在页面上也会自己更新
    clearInterval(this.timer);
    this.timer = setInterval(() => this.load(), 15000);
  },
  onHide() {
    clearInterval(this.timer);
  },
  onUnload() {
    clearInterval(this.timer);
  },
  load() {
    const orders = store.getOrders();
    const now = Date.now();
    // 进行中的最新一单；都结束了就只列历史
    const order = orders.find(o => store.stageOf(o, now).active) || null;
    const history = orders.filter(o => o !== order).slice(0, 5).map(o => brief(o, now));
    if (!order) {
      this.setData({ order: null, st: null, history, total: orders.length });
      return;
    }
    const st = store.stageOf(order, now);
    this.setData({
      order,
      st,
      store: data.findStore(order.storeId),
      steps: st.names.map((name, i) => ({ name, done: i <= st.stage })),
      history,
      total: orders.length
    });
  },
  onCall() {
    host.call(this.data.store.phone);
  },
  onNav() {
    host.openMap(this.data.store);
  },
  onCancel() {
    const o = this.data.order;
    wx.showModal({
      title: '取消订单',
      content: `取消后 ¥${o.pay} 将原路退回${o.payBy === 'wallet' ? '咖啡钱包' : '微信'}，优惠券与积分一并退回`,
      confirmText: '确认取消',
      cancelText: '再想想',
      success: r => {
        if (!r.confirm) return;
        if (!store.cancelOrder(o.id)) {
          wx.showToast({ title: '咖啡师已开始制作，无法取消', icon: 'none' });
        } else {
          wx.showToast({ title: '订单已取消', icon: 'none' });
        }
        this.load();
      }
    });
  },
  onConfirm() {
    const o = this.data.order;
    store.updateOrder(o.id, { status: 'done', doneAt: Date.now() });
    wx.showToast({ title: o.mode === 'pickup' ? '已取餐，祝你享用愉快' : '已确认收货', icon: 'none' });
    this.load();
  },
  onDetail(e) {
    const id = (e && e.currentTarget.dataset.id) || this.data.order.id;
    wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + id });
  },
  onAgain() {
    const n = store.reorder(this.data.order);
    if (!n) return;
    wx.navigateTo({ url: '/pages/cart/cart' });
  },
  onAll() {
    wx.navigateTo({ url: '/pages/orders/orders' });
  },
  onGoMenu() {
    wx.switchTab({ url: '/pages/menu/menu' });
  }
});
