const D = require('../../utils/date');
const F = require('../../utils/flights');
const ID = require('../../utils/idcard');
const store = require('../../utils/store');
const order = require('../../utils/order');
const P = require('./price');
const app = getApp();

// 填写订单：乘机人（从乘机人页经 EventChannel 选回）、联系人、保险、价格明细 → 提交 → 支付
Page({
  data: {
    legs: [], pax: [], contact: { name: '', phone: '' }, ins: { acc: true, delay: false }, INS: P.INS,
    rows: [], total: 0, showDetail: false, errors: {}, cashier: false, orderId: '', expireAt: 0, payReason: ''
  },
  onLoad() {
    let b = app.globalData.booking;
    if (!b || !b.legs.length) {
      // 直接打开本页（无头走查 / 分享进入）：给明天杭州→北京的第一班特价经济舱
      const date = D.addDays(D.today(), 1), f = F.genFlights('HGH', 'BJS', date)[0], c = f.econ[0];
      b = app.globalData.booking = { trip: 'one', legs: [{
        flight: { no: f.no, airline: f.airline, airlineName: f.airlineName, logo: f.logo, date, fromCode: 'HGH', toCode: 'BJS', fromName: f.fromName,
          toName: f.toName, depText: f.depText, arrText: f.arrText, dep: f.dep, cross: f.cross, durText: f.durText, depAirport: f.depAirport + f.depTerm,
          arrAirport: f.arrAirport + f.arrTerm, aircraft: f.aircraft, stop: f.stop, dist: f.dist, full: f.full },
        cabin: { id: c.id, name: c.name, price: c.price, rule: c.rule, bag: c.bag, discount: c.discount } }] };
    }
    const legs = b.legs.map((l, i) => Object.assign({}, l, { tag: b.legs.length > 1 ? (i ? '返程' : '去程') : '单程', dateText: D.md(l.flight.date) + ' ' + D.weekLabel(l.flight.date) }));
    const self = store.passengers().find(p => p.self);
    this.selected = this.selected || (self ? [self.id] : []);
    this.setData({ legs, contact: store.contact() });
    this.refreshPax();
  },
  onShow() { this.refreshPax(); },
  refreshPax() {
    const all = store.passengers();
    this.selected = (this.selected || []).filter(id => all.some(p => p.id === id));
    const pax = this.selected.map(id => all.find(p => p.id === id)).map(p => Object.assign({}, p, {
      age: ID.ageType(p.idType, p.idNo), typeName: ID.TYPES[p.idType], masked: ID.mask(p.idNo) }));
    this.setData({ pax });
    this.recalc();
  },
  recalc() {
    const r = P.calc(this.data.legs, this.data.pax, this.data.ins);
    this.setData({ rows: r.rows, total: r.total });
  },
  choosePax() {
    wx.navigateTo({
      url: '/pages/passengers/passengers?mode=select',
      events: { pickPassengers: (res) => { this.selected = res.ids; this.refreshPax(); this.setData({ 'errors.pax': '' }); } },
      success: (r) => r.eventChannel.emit('init', { ids: this.selected })
    });
  },
  removePax(e) {
    this.selected = this.selected.filter(id => id !== e.currentTarget.dataset.id);
    this.refreshPax();
  },
  onContact(e) { this.setData({ ['contact.' + e.currentTarget.dataset.k]: e.detail.value, ['errors.' + e.currentTarget.dataset.k]: '' }); },
  onIns(e) { this.setData({ ['ins.' + e.currentTarget.dataset.k]: e.detail.value }); this.recalc(); },
  toggleDetail() { this.setData({ showDetail: !this.data.showDetail }); },
  showRule(e) {
    const l = this.data.legs[e.currentTarget.dataset.i];
    const rows = F.ruleRows(l.cabin.rule, l.cabin.price);
    wx.showModal({
      title: l.cabin.name + ' 退改规则',
      content: rows.map(r => r.win + '：退票 ¥' + r.refund + '，改期 ¥' + r.change).join('\n') + '\n' + F.RULES[l.cabin.rule].endorse,
      showCancel: false
    });
  },
  validate() {
    const d = this.data, errors = {};
    if (!d.pax.length) errors.pax = '请至少选择 1 位乘机人';
    else if (d.pax.length > 9) errors.pax = '一个订单最多 9 位乘机人';
    else if (!d.pax.some(p => p.age === '成人')) errors.pax = '儿童、婴儿需由成人陪同乘机，请添加一位成人';
    else if (d.pax.filter(p => p.age === '婴儿').length > d.pax.filter(p => p.age === '成人').length) errors.pax = '每位成人最多携带 1 名婴儿';
    if (!String(d.contact.name || '').trim()) errors.name = '请填写联系人姓名';
    const pe = ID.checkPhone(d.contact.phone);
    if (pe) errors.phone = pe;
    this.setData({ errors });
    return !Object.keys(errors).length;
  },
  submit() {
    if (!this.validate()) return wx.showToast({ title: '请完善订单信息', icon: 'none' });
    store.saveContact(this.data.contact);
    const o = order.create(this.data.legs.map(l => ({ flight: l.flight, cabin: l.cabin })), this.data.pax, this.data.contact, this.data.ins,
      { rows: this.data.rows, total: this.data.total });
    this.setData({ orderId: o.id, expireAt: o.expireAt });
    order.requestPay(o, (paid) => this.onPaid({ detail: { id: paid.id } }), (err) => {
      console.log('requestPayment 不可用：', err && err.errMsg);
      this.setData({ cashier: true, payReason: '当前环境暂不支持微信支付，已切换到模拟收银台' });
    });
  },
  onPaid(e) {
    this.setData({ cashier: false });
    app.globalData.booking = null;
    wx.redirectTo({ url: '/pages/pay-result/pay-result?id=' + e.detail.id });
  },
  onCashierClose() {
    this.setData({ cashier: false });
    wx.showToast({ title: '订单已保留，可在订单页继续支付', icon: 'none' });
    setTimeout(() => wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + this.data.orderId }), 800);
  },
  onTimeout() {
    order.cancel(this.data.orderId);
    this.setData({ cashier: false });
    wx.showModal({ title: '支付超时', content: '订单已自动取消，请重新预订。', showCancel: false, success: () => wx.navigateBack() });
  }
});
