const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: {
    balance: '0.00',
    couponCount: 0,
    saved: '0.00',
    amounts: [{ v: 20 }, { v: 50 }, { v: 100 }, { v: 200, gift: 10 }, { v: 300, gift: 20 }, { v: 500, gift: 40 }],
    aIdx: 2,
    custom: '',
    customSeq: 0,
    pays: [
      { name: '微信支付', glyph: '微', bg: 'linear-gradient(135deg, #3FD779, #07C160)' },
      { name: '云闪付', glyph: '云', bg: 'linear-gradient(135deg, #FF7A7A, #E5383B)' }
    ],
    pIdx: 0,
    autoPay: false,
    autoLine: store.AUTO_LINE,
    autoAmount: store.AUTO_AMOUNT,
    payText: '100',
    logs: []
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.load();
  },
  load() {
    const w = store.getWallet();
    const saved = store.getOrders().reduce((s, o) => s + (o.cut || 0), 0);
    this.setData({
      balance: w.balance.toFixed(2),
      couponCount: store.validCoupons().length,
      saved: store.round2(saved).toFixed(2),
      autoPay: store.autoPay(),
      logs: w.logs.map(l => Object.assign({}, l, { vText: (l.amount > 0 ? '+' : '-') + Math.abs(l.amount).toFixed(2) }))
    });
  },
  amount() {
    if (this.data.aIdx >= 0) { const a = this.data.amounts[this.data.aIdx]; return a.v + (a.gift || 0); }
    return Number(this.data.custom) || 0;
  },
  pay() { return this.data.aIdx >= 0 ? this.data.amounts[this.data.aIdx].v : (Number(this.data.custom) || 0); },
  onAmount(e) {
    const changed = this.data.custom !== '';
    this.setData({ aIdx: Number(e.currentTarget.dataset.idx), custom: '', customSeq: this.data.customSeq + (changed ? 1 : 0) });
    this.setData({ payText: String(this.pay()) });
  },
  onCustomFocus() { this.setData({ aIdx: -1, payText: String(Number(this.data.custom) || 0) }); },
  onCustom(e) { this.setData({ aIdx: -1, custom: e.detail.value }); this.setData({ payText: String(Number(e.detail.value) || 0) }); },
  onPay(e) { this.setData({ pIdx: Number(e.currentTarget.dataset.idx) }); },
  onAuto(e) {
    store.setAutoPay(e.detail.value);
    this.setData({ autoPay: !!e.detail.value });
    if (e.detail.value) wx.showToast({ title: '已开通免密自动充值', icon: 'none' });
  },
  onCoupons() { wx.navigateTo({ url: '/pages/coupons/coupons' }); },
  onRecharge() {
    const pay = this.pay();
    if (pay < 10 || pay > 2000) { wx.showToast({ title: '充值金额需在 10~2000 元', icon: 'none' }); return; }
    const get = this.amount();
    const channel = this.data.pays[this.data.pIdx].name;
    host.pay(pay, '易充电余额充值' + (get > pay ? '，到账 ¥' + get : ''), channel).then(ok => {
      if (!ok) return;
      store.changeWallet(get, channel + '充值' + (get > pay ? '（含赠送 ' + (get - pay) + ' 元）' : ''), 'recharge');
      this.load();
      wx.showToast({ title: '充值成功，到账 ¥' + get, icon: 'none' });
    });
  }
});
