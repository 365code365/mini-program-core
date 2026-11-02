const { COUPONS } = require('../../utils/data.js');
const store = require('../../utils/store.js');

const PRESETS = [{ v: '200', gift: 0 }, { v: '500', gift: 5 }, { v: '1000', gift: 20 }];
function gift(n) { return n >= 1000 ? 20 : n >= 500 ? 5 : 0; }

Page({
  data: {
    tab: 'card', card: {}, balance: '0.00', logs: [], presets: PRESETS, amount: '500', custom: '', err: '', giftText: '（送 ¥5）',
    cpTabs: [{ k: 'unused', t: '可使用' }, { k: 'used', t: '已使用' }, { k: 'expired', t: '已过期' }, { k: 'center', t: '领券中心' }],
    cpTab: 'unused', mine: [], center: [], unusedCount: 0, cashier: false, payAmount: '0.00', orderNo: ''
  },
  onLoad(q) {
    if (q.tab === 'coupon') this.setData({ tab: 'coupon', cpTab: 'center' });
    wx.setNavigationBarTitle({ title: '油卡与优惠券' });
  },
  onShow() { this.loadCard(); this.loadCoupons(); },

  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.t }); },
  loadCard() {
    const c = store.card();
    this.setData({ card: c, balance: c.balance.toFixed(2), logs: c.logs.map((l) => Object.assign({}, l, { time: store.fmt(l.at, true), plus: parseFloat(l.delta) > 0 })) });
  },
  pickPreset(e) { this.setAmount(e.currentTarget.dataset.v, ''); },
  // type="digit"：过滤成「数字 + 一个小数点 + 最多两位小数」
  onCustom(e) {
    let v = String(e.detail.value).replace(/[^\d.]/g, '');
    const i = v.indexOf('.');
    if (i >= 0) v = v.slice(0, i + 1) + v.slice(i + 1).replace(/\./g, '').slice(0, 2);
    this.setAmount(v, v);
    return v;
  },
  setAmount(v, custom) {
    const g = gift(parseFloat(v) || 0);
    this.setData({ amount: v, custom, err: '', giftText: g ? `（送 ¥${g}）` : '' });
  },
  recharge() {
    const n = parseFloat(this.data.amount);
    if (!(n >= 50)) { this.setData({ err: '单次充值最低 50 元' }); return; }
    if (n > 5000) { this.setData({ err: '单次充值最高 5000 元' }); return; }
    const no = 'CZ' + Date.now();
    this.setData({ payAmount: store.money(n), orderNo: no });
    const ts = String(Math.floor(Date.now() / 1000));
    wx.requestPayment({
      timeStamp: ts, nonceStr: 'cz' + ts, package: 'prepay_id=wx' + no, signType: 'RSA', paySign: 'mock-sign',
      success: () => this.onPaid(), fail: () => this.setData({ cashier: true })
    });
  },
  onPaid() {
    const n = parseFloat(this.data.payAmount);
    const g = gift(n);
    store.cardChange(n, '油卡充值');
    if (g) store.cardChange(g, `充值赠送（充 ${n} 送 ${g}）`);
    this.setData({ cashier: false });
    wx.vibrateShort({ type: 'light' });
    wx.showToast({ title: `已到账 ¥${store.money(n + g)}`, icon: 'success' });
    this.loadCard();
  },
  onPayCancel() { this.setData({ cashier: false }); wx.showToast({ title: '已取消充值', icon: 'none' }); },

  switchCp(e) { this.setData({ cpTab: e.currentTarget.dataset.k }); this.loadCoupons(); },
  loadCoupons() {
    const all = store.coupons();
    const held = all.filter((c) => c.status === 'unused').map((c) => c.tpl);
    this.setData({
      unusedCount: all.filter((c) => c.status === 'unused').length,
      mine: all.filter((c) => c.status === this.data.cpTab).map((c) => Object.assign({}, c, { expireText: store.fmt(c.expireAt, true) })),
      center: COUPONS.map((c) => Object.assign({}, c, { held: held.indexOf(c.id) >= 0 }))
    });
  },
  claim(e) {
    const c = store.claim(e.currentTarget.dataset.id);
    if (!c) { wx.showToast({ title: '已领取过，用完可再领', icon: 'none' }); return; }
    wx.showToast({ title: '领取成功', icon: 'success' });
    this.loadCoupons();
  },
  useNow() { wx.switchTab({ url: '/pages/index/index' }); }
});
