const { OILS, stationById } = require('../../utils/data.js');
const store = require('../../utils/store.js');

const MAX = 3000;   // 单笔上限（元）
const MIN = 10;

// 数字键盘的输入规则：只有一个小数点、最多两位小数、不超过上限、不出现 "007" 这种前导零
function applyKey(cur, k) {
  if (k === 'clear') return '';
  if (k === 'del') return cur.slice(0, -1);
  if (k === '.') {
    if (cur.indexOf('.') >= 0) return cur;
    return (cur || '0') + '.';
  }
  const i = cur.indexOf('.');
  if (i >= 0 && cur.length - i > 2) return cur;          // 已有两位小数
  const next = cur === '0' ? k : cur + k;                 // "0" 后面接数字 → 替换
  if (parseFloat(next) > MAX) return null;               // 超上限
  return next;
}

Page({
  data: {
    st: {}, oils: [], oil: '', guns: [], gun: 0, amount: '', quick: ['100', '200', '300', '500'], keys: ['1', '2', '3', '4', '5', '6', '7', '8', '9', '.', '0', 'del'],
    max: MAX, kb: false, calc: null, err: {}, usable: [], couponId: '', couponText: '', cpShow: false, cashier: false, orderNo: '', cardBalance: 0,
    net: { isConnected: true }
  },

  onLoad(q) {
    const st = stationById(q.id) || stationById('s1');
    this.st = st;
    const oils = OILS.filter((o) => st.prices[o]);
    const want = decodeURIComponent(q.oil || store.oilPref());
    this.setData({ st, oils });
    this.pickOil({ currentTarget: { dataset: { oil: oils.indexOf(want) >= 0 ? want : oils[0] } } });
    this.setData({ kb: true });
  },
  onShow() {
    this.setData({ net: getApp().globalData.net, cardBalance: store.card().balance });
  },
  onNetChange(n) { this.setData({ net: n }); },

  pickOil(e) {
    const oil = e.currentTarget.dataset.oil;
    this.setData({ oil, guns: this.st.guns[oil] || [], gun: 0, 'err.gun': '' });
    this.recalc();
  },
  pickGun(e) { this.setData({ gun: e.currentTarget.dataset.gun, 'err.gun': '' }); },
  pickQuick(e) { this.setData({ amount: e.currentTarget.dataset.v, 'err.amount': '' }); this.autoCoupon(); },

  openKb() { this.setData({ kb: true, cpShow: false }); },
  closeKb() { this.setData({ kb: false }); },
  confirmKb() {
    if (!this.data.amount) { wx.showToast({ title: '请输入加油金额', icon: 'none' }); return; }
    this.setData({ kb: false });
    this.autoCoupon();
  },
  press(e) {
    const next = applyKey(this.data.amount, e.currentTarget.dataset.k);
    wx.vibrateShort({ type: 'light' });
    if (next === null) { wx.showToast({ title: `单笔最高 ¥${MAX}`, icon: 'none' }); return; }
    if (next === this.data.amount) return;
    this.setData({ amount: next, 'err.amount': '' });
    this.recalc();
  },

  // 金额变了：自动挑一张减得最多的可用券
  autoCoupon() {
    const amt = parseFloat(this.data.amount) || 0;
    const cur = store.usable(amt).find((c) => c.id === this.data.couponId);
    const best = store.usable(amt)[0];
    this.setData({ couponId: cur ? cur.id : best ? best.id : '' });
    this.recalc();
  },
  recalc() {
    const amt = parseFloat(this.data.amount);
    if (!(amt > 0) || !this.data.oil) { this.setData({ calc: null, couponText: '' }); return; }
    const pr = this.st.prices[this.data.oil];
    const usable = store.usable(amt);
    let cp = usable.find((c) => c.id === this.data.couponId);
    if (!cp && this.data.couponId) this.setData({ couponId: '' });
    const liters = amt / pr.list;
    const discount = liters * (pr.list - pr.ours);
    const cut = cp ? cp.cut : 0;
    const pay = Math.max(0.01, amt - discount - cut);
    this.setData({
      usable: usable.map((c) => Object.assign({}, c, { expireText: store.fmt(c.expireAt) })),
      couponText: cp ? `-¥${cut.toFixed(2)}` : usable.length ? `${usable.length} 张可用` : '暂无可用',
      calc: { liters: liters.toFixed(2), list: pr.list.toFixed(2), ours: pr.ours.toFixed(2), discount: store.money(discount), couponCut: cut, pay: store.money(pay), saved: store.money(discount + cut) }
    });
  },

  openCoupon() { this.setData({ cpShow: true, kb: false }); },
  closeCoupon() { this.setData({ cpShow: false }); },
  onCoupon(e) { this.setData({ couponId: e.detail.value }); this.recalc(); },
  pickCoupon(e) { this.setData({ couponId: e.currentTarget.dataset.id, cpShow: false }); this.recalc(); },

  validate() {
    const err = {};
    const amt = parseFloat(this.data.amount);
    if (!this.data.gun) err.gun = '请选择油枪号';
    if (!(amt > 0)) err.amount = '请输入加油金额';
    else if (amt < MIN) err.amount = `最低加油金额 ¥${MIN}`;
    this.setData({ err });
    return !Object.keys(err).length;
  },
  submit() {
    if (!this.validate()) { this.setData({ kb: !this.data.amount }); return; }
    if (!this.data.net.isConnected) { wx.showToast({ title: '网络不可用', icon: 'none' }); return; }
    const d = this.data;
    wx.showModal({
      title: '请核对油枪',
      content: `${d.oil} · ${d.gun} 号油枪 · 加油 ¥${d.amount}\n确认加油员已按此金额加注`,
      confirmText: '确认付款',
      success: (r) => { if (r.confirm) this.pay(); }
    });
  },
  pay() {
    const d = this.data;
    const cp = d.usable.find((c) => c.id === d.couponId);
    this.pending = store.makeRecord({ station: this.st, oil: d.oil, gun: d.gun, amount: parseFloat(d.amount), couponCut: cp ? cp.cut : 0, way: 'wx' });
    this.pendingCoupon = cp ? cp.id : '';
    this.setData({ orderNo: this.pending.no });
    const ts = String(Math.floor(Date.now() / 1000));
    wx.requestPayment({
      timeStamp: ts, nonceStr: 'yj' + ts, package: 'prepay_id=wx' + this.pending.no, signType: 'RSA', paySign: 'mock-sign',
      success: () => this.finish('wx'),
      fail: () => this.setData({ cashier: true, cardBalance: store.card().balance })
    });
  },
  onPaid(e) { this.setData({ cashier: false }); this.finish(e.detail.way); },
  onPayCancel() { this.setData({ cashier: false }); wx.showToast({ title: '已取消支付，可重新发起', icon: 'none' }); },
  finish(way) {
    const r = Object.assign({}, this.pending, { way });
    if (way === 'card') store.cardChange(-parseFloat(r.pay), `${this.st.name} ${r.oil} 加油`);
    if (this.pendingCoupon) store.useCoupon(this.pendingCoupon);
    store.addRecord(r);
    wx.vibrateShort({ type: 'medium' });
    wx.redirectTo({ url: '/pages/pay-result/pay-result?id=' + r.id });
  }
});
