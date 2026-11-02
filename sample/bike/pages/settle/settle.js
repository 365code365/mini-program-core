// 行程结算：费用明细 → 余额 / 微信支付 → 支付成功（减碳、查看行程）
const fee = require('../../utils/fee.js');
const store = require('../../utils/store.js');
const pay = require('../../utils/pay.js');

Page({
  data: {
    trip: null,
    view: null,
    method: 'balance',
    balance: '0.00',
    enough: true,
    paying: false,
    cashier: { show: false, amount: 0, title: '' }
  },

  onLoad(q) {
    this._id = q.id;
    this.load();
  },

  load() {
    const t = store.trip(this._id);
    if (!t) { this.setData({ trip: null }); return; }
    const w = store.wallet();
    const enough = w.balance >= t.paid;
    this.setData({
      trip: t,
      view: {
        name: fee.PRICING[t.type].name,
        dur: fee.fmtMin(t.sec),
        km: (t.meters / 1000).toFixed(2),
        start: t.start.toFixed(2), extra: t.extra.toFixed(2), total: t.total.toFixed(2),
        discount: t.discount.toFixed(2), paid: t.paid.toFixed(2),
        carbon: (t.meters / 1000 * 0.15).toFixed(2),
        time: store.fmtTime(t.startAt) + ' - ' + store.fmtTime(t.endAt, false),
        payBy: { balance: '余额支付', wechat: '微信支付', card: '骑行卡抵扣', free: '免费（2 分钟内结束）' }[t.payBy] || ''
      },
      balance: w.balance.toFixed(2),
      enough,
      method: this.data.method === 'balance' && !enough ? 'wechat' : this.data.method
    });
  },

  onMethod(e) {
    const m = e.detail.value;
    if (m === 'balance' && !this.data.enough) {
      wx.showToast({ title: '余额不足，请先充值或使用微信支付', icon: 'none' });
      this.setData({ method: 'wechat' });
      return;
    }
    this.setData({ method: m });
  },

  onPay() {
    const t = this.data.trip;
    if (!t || t.status === 'paid' || this.data.paying) return;
    if (this.data.method === 'balance') {
      const w = store.wallet();
      if (w.balance < t.paid) { wx.showToast({ title: '余额不足', icon: 'none' }); return; }
      w.balance -= t.paid;
      store.addTxn(w, fee.PRICING[t.type].name.replace('青桔', '') + '骑行 · ' + t.to, -t.paid);
      store.saveWallet(w);
      this.done('balance');
      return;
    }
    pay.pay(this, { amount: t.paid, title: '骑行费用' }, () => {
      const w = store.wallet();
      // 微信支付不动余额，只记一笔账单
      store.addTxn(w, '骑行费（微信支付） · ' + t.to, -t.paid);
      store.saveWallet(w);
      this.done('wechat');
    });
  },

  done(by) {
    store.updateTrip(this._id, { status: 'paid', payBy: by, paidAt: Date.now() });
    wx.showToast({ title: '支付成功', icon: 'success' });
    this.load();
  },

  onCashierConfirm() { pay.confirm(this); },
  onCashierCancel() { pay.cancel(this); },
  goRecharge() { wx.navigateTo({ url: '/pages/wallet/wallet' }); },
  onShow() { if (this._id && this.data.trip) this.load(); },
  goDetail() { wx.redirectTo({ url: '/pages/trip/trip?id=' + this._id }); },
  goHome() { wx.reLaunch({ url: '/pages/index/index' }); }
});
