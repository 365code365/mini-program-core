const store = require('../../utils/store.js');
Page({
  data: {
    tabs: [{ k: 'all', n: '全部' }, { k: 'paid', n: '制作中' }, { k: 'shipping', n: '配送中' }, { k: 'done', n: '已签收' }, { k: 'cancel', n: '已取消' }],
    tab: 'all', list: []
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    // 「我的」页点了某个状态：switchTab 带不了参数，放在 storage 里
    const want = wx.getStorageSync(store.K.orderTab);
    if (want) { wx.removeStorageSync(store.K.orderTab); this.setData({ tab: want }); }
    this.refresh();
  },
  refresh() {
    const list = store.getOrders().filter(o => this.data.tab === 'all' || o.status === this.data.tab)
      .map(o => {
        const items = o.items.map(store.expand);
        const st = store.stageOf(o);
        return Object.assign({}, o, { items, st: st.text, canCancel: st.canCancel, canRate: st.canRate, qty: items.reduce((s, x) => s + x.qty, 0) });
      });
    this.setData({ list });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.refresh(); },
  open(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?no=' + e.currentTarget.dataset.no }); },
  rate(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?rate=1&no=' + e.currentTarget.dataset.no }); },
  cancel(e) {
    const no = String(e.currentTarget.dataset.no);
    const o = store.findOrder(no);
    wx.showModal({
      title: '取消订单', content: '取消后 ¥' + o.total + ' 将原路退回微信' + (o.couponTitle ? '，优惠券一并退回' : '') + '。', confirmText: '确认取消', confirmColor: '#ef4f76',
      success: r => {
        if (!r.confirm) return;
        const ok = store.cancelOrder(no);
        this.refresh();
        wx.showToast({ title: ok ? '已取消，退款 1-3 天到账' : '花艺师已开始制作，无法取消', icon: 'none' });
      }
    });
  },
  again(e) {
    store.reorder(store.findOrder(String(e.currentTarget.dataset.no)));
    wx.switchTab({ url: '/pages/cart/cart' });
  },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
