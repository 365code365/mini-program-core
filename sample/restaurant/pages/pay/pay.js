const S = require('../../utils/store.js');

Page({
  data: { order: null, left: '15:00', urgent: false, expired: false, method: 'wx', balanceText: '0', balanceEnough: true, paying: false, payText: '0' },
  onLoad(q) {
    // 没带 id（比如直接打开这一页）时取最近一笔待支付订单
    if (!q.id) { const u = S.getOrders().find(o => o.status === 'unpaid') || S.getOrders()[0]; q = { id: u && u.id }; }
    this.id = q.id;
    const o = S.getOrder(q.id);
    if (!o) return;
    // 已经付过的订单不再停留在支付页
    if (o.status === 'paid' || o.status === 'done') { wx.redirectTo({ url: '/pages/progress/progress?id=' + o.id }); return; }
    const bal = S.getBalance();
    this.setData({
      order: { id: o.id, no: o.no, table: o.table, count: o.items.reduce((n, i) => n + i.qty, 0), name: o.items[0].name },
      payText: S.yuan(o.pay), balanceText: S.yuan(bal), balanceEnough: bal >= o.pay
    });
    this.tick();
    this.timer = setInterval(() => this.tick(), 1000);
  },
  onUnload() { clearInterval(this.timer); },
  // 倒计时以订单里的 deadline 为准（离开再回来不会重置）；到点把订单置为已取消
  tick() {
    const o = S.getOrder(this.id);
    if (!o) return;
    if (o.status === 'canceled') {
      clearInterval(this.timer);
      this.setData({ expired: true, left: '00:00', urgent: false });
      return;
    }
    if (o.status !== 'unpaid') { clearInterval(this.timer); return; }
    const ms = Math.max(0, o.deadline - Date.now());
    const sec = Math.ceil(ms / 1000);
    const left = S.pad(Math.floor(sec / 60)) + ':' + S.pad(sec % 60);
    if (left !== this.data.left || (sec <= 60) !== this.data.urgent) this.setData({ left, urgent: sec <= 60 });
  },
  onMethod(e) {
    const m = e.currentTarget.dataset.m;
    if (m === 'card' && !this.data.balanceEnough) { wx.showToast({ title: '饭卡余额不足', icon: 'none' }); return; }
    this.setData({ method: m });
  },
  pay() {
    if (this.data.paying || this.data.expired) return;
    this.setData({ paying: true });
    wx.showLoading({ title: '支付中' });
    setTimeout(() => {
      wx.hideLoading();
      const o = S.getOrder(this.id);
      if (!o || o.status !== 'unpaid') { this.setData({ paying: false }); this.tick(); return; }
      if (this.data.method === 'card') S.setBalance(S.getBalance() - o.pay);
      S.updateOrder(this.id, { status: 'paid', paidAt: Date.now(), payMethod: this.data.method === 'card' ? '饭卡' : '微信支付', stage: 0 });
      clearInterval(this.timer);
      wx.redirectTo({ url: '/pages/progress/progress?id=' + this.id });
    }, 800);
  },
  cancelOrder() {
    wx.showModal({
      title: '取消订单', content: '取消后菜品不会保留，确定取消吗？', confirmText: '取消订单', cancelText: '再想想',
      success: r => {
        if (!r.confirm) return;
        S.updateOrder(this.id, { status: 'canceled', cancelReason: '用户取消' });
        this.tick();
      }
    });
  },
  // 超时 / 取消后：把菜品放回购物车重新下单
  reorder() {
    S.reorder(S.getOrder(this.id));
    wx.switchTab({ url: '/pages/menu/menu' });
  }
});
