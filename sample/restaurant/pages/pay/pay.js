const S = require('../../utils/store.js');
const host = require('../../utils/host.js');

Page({
  data: { order: null, missing: false, left: '15:00', urgent: false, expired: false, method: 'wx', balanceText: '0', balanceEnough: true, paying: false, payText: '0' },
  onLoad(q) {
    // 没带 id（比如直接打开这一页）时取最近一笔待支付订单
    let id = q && q.id;
    if (!id) { const u = S.getOrders().find(o => o.status === 'unpaid'); id = u && u.id; }
    this.id = id;
    const o = id ? S.getOrder(id) : null;
    if (!o) { this.setData({ missing: true }); return; }
    // 已经付过的订单不再停留在支付页
    if (o.status === 'paid' || o.status === 'done') { wx.redirectTo({ url: '/pages/progress/progress?id=' + o.id }); return; }
    this.setData({
      order: { id: o.id, no: o.no, table: o.table, count: o.items.reduce((n, i) => n + i.qty, 0), name: o.items[0].name },
      payText: S.yuan(o.pay)
    });
    this.syncBalance();
    this.tick();
    this.timer = setInterval(() => this.tick(), 1000);
  },
  // 去饭卡页充值回来，余额要跟上
  onShow() { if (this.data.order) this.syncBalance(); },
  onUnload() { clearInterval(this.timer); },
  syncBalance() {
    const o = S.getOrder(this.id);
    if (!o) return;
    const bal = S.getBalance();
    const enough = bal >= o.pay;
    this.setData({ balanceText: S.yuan(bal), balanceEnough: enough, method: this.data.method === 'card' && !enough ? 'wx' : this.data.method });
  },
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
    if (m === 'card' && !this.data.balanceEnough) { this.askRecharge(); return; }
    this.setData({ method: m });
  },
  askRecharge() {
    const o = S.getOrder(this.id);
    const short = o.pay - S.getBalance();
    wx.showModal({
      title: '饭卡余额不足',
      content: '饭卡余额 ¥' + S.yuan(S.getBalance()) + '，本单需支付 ¥' + S.yuan(o.pay) + '，还差 ¥' + S.yuan(short),
      confirmText: '去充值',
      success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/card/card?need=' + short }); }
    });
  },
  // 微信支付先走 host.pay（宿主没有支付能力时退回页内确认框）；饭卡支付先确认再扣款
  pay() {
    if (this.data.paying || this.data.expired || !this.data.order) return;
    const o = S.getOrder(this.id);
    if (!o || o.status !== 'unpaid') { this.tick(); return; }
    if (this.data.method === 'card') {
      if (S.getBalance() < o.pay) { this.askRecharge(); return; }
      wx.showModal({
        title: '饭卡支付',
        content: '饭卡余额 ¥' + S.yuan(S.getBalance()) + '，本单支付 ¥' + S.yuan(o.pay),
        confirmText: '确认支付',
        success: r => { if (r.confirm) this.finish('card'); }
      });
      return;
    }
    this.setData({ paying: true });
    host.pay(o.pay, '禾味食堂 · 桌号 ' + o.table + ' · ' + this.data.order.count + ' 份').then(ok => {
      this.setData({ paying: false });
      if (ok) this.finish('wx');
      else wx.showToast({ title: '已取消支付', icon: 'none' });
    });
  },
  finish(method) {
    const r = S.payOrder(this.id, method);
    if (r.err) {
      wx.showToast({ title: r.err, icon: 'none' });
      this.syncBalance();
      this.tick();
      return;
    }
    clearInterval(this.timer);
    wx.showToast({ title: '支付成功', icon: 'success' });
    wx.redirectTo({ url: '/pages/progress/progress?id=' + this.id });
  },
  cancelOrder() {
    wx.showModal({
      title: '取消订单', content: '取消后菜品不会保留，确定取消吗？', confirmText: '取消订单', cancelText: '再想想',
      success: r => {
        if (!r.confirm) return;
        S.cancelOrder(this.id);
        this.tick();
      }
    });
  },
  // 超时 / 取消后：把菜品放回购物车重新下单
  reorder() {
    S.reorder(S.getOrder(this.id));
    wx.switchTab({ url: '/pages/menu/menu' });
  },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); }
});
