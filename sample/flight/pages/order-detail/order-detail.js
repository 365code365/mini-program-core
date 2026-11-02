const D = require('../../utils/date');
const ID = require('../../utils/idcard');
const store = require('../../utils/store');
const order = require('../../utils/order');
const cal = require('../../utils/calendar');

const REASONS = ['行程变更，不再出行', '航班延误或取消', '身体原因，无法乘机', '买错了 / 重复购买', '其他原因'];

// 订单详情：行程单、乘机人与票号、价格、进度；按状态给出支付 / 取消 / 退票 / 改期 / 删除
Page({
  data: { o: null, legs: [], pax: [], log: [], left: '', cashier: false, payReason: '' },
  onLoad(q) { this.id = q.id || (store.orders()[0] || {}).id; },
  onShow() { this.refresh(); },
  onUnload() { clearInterval(this._timer); },
  refresh() {
    let o = store.order(this.id);
    if (!o) return this.setData({ o: null });
    if (order.expired(o)) o = order.cancel(o.id);
    this.o = o;
    this.setData({
      o: { id: o.id, status: o.status, statusText: order.STATUS[o.status], total: o.total, rows: o.rows, contact: o.contact,
        refund: o.refund || null, changeFee: o.changeFee || 0, created: D.stamp(o.createdAt), expireAt: o.expireAt,
        ins: [o.ins.acc ? '航空意外险' : '', o.ins.delay ? '航班延误险' : ''].filter(Boolean).join('、') || '未购买' },
      legs: o.legs.map((l, i) => ({ k: i, tag: o.legs.length > 1 ? (i ? '返程' : '去程') : '单程', f: l.flight, c: l.cabin,
        dateText: D.md(l.flight.date) + ' ' + D.weekLabel(l.flight.date) })),
      pax: o.pax.map(p => ({ name: p.name, age: p.age, idText: ID.TYPES[p.idType] + ' ' + ID.mask(p.idNo), ticket: p.ticket || '' })),
      log: o.log.slice().reverse().map((x, i) => ({ k: i, t: D.stamp(x.t), text: x.text }))
    });
    clearInterval(this._timer);
    if (o.status === 'unpaid') { this.tick(); this._timer = setInterval(() => this.tick(), 1000); }
  },
  tick() {
    const ms = this.o.expireAt - Date.now();
    if (ms <= 0) { clearInterval(this._timer); return this.refresh(); }
    const s = Math.floor(ms / 1000);
    this.setData({ left: D.pad(Math.floor(s / 60)) + ':' + D.pad(s % 60) });
  },
  pay() {
    order.requestPay(this.o, () => this.refresh(), (err) => { console.log('requestPayment 不可用：', err && err.errMsg); this.setData({ cashier: true, payReason: '当前环境暂不支持微信支付，已切换到模拟收银台' }); });
  },
  onPaid() {
    this.setData({ cashier: false });
    wx.redirectTo({ url: '/pages/pay-result/pay-result?id=' + this.o.id });
  },
  onCashierClose() { this.setData({ cashier: false }); },
  onTimeout() { this.setData({ cashier: false }); this.refresh(); },
  cancel() {
    wx.showModal({
      title: '取消订单', content: '取消后座位将释放，确定取消吗？', confirmText: '取消订单', cancelText: '再想想',
      success: (r) => { if (r.confirm) { order.cancel(this.o.id); this.refresh(); } }
    });
  },
  refund() {
    wx.showActionSheet({
      itemList: REASONS,
      success: (r) => {
        const q = order.refundQuote(this.o);
        wx.showModal({
          title: '确认退票',
          content: '距起飞约 ' + Math.max(0, Math.floor(q.hours)) + ' 小时，按所购运价规则收取退票手续费 ¥' + q.fee + '，预计退款 ¥' + q.back + '，1~7 个工作日原路退回。',
          confirmText: '确认退票', confirmColor: '#f04438',
          success: (m) => { if (m.confirm) { order.refund(this.o.id, REASONS[r.tapIndex]); wx.vibrateShort({ type: 'light' }); this.refresh(); wx.showToast({ title: '退票申请已提交', icon: 'none' }); } }
        });
      }
    });
  },
  change() {
    const f = this.o.legs[0].flight;
    const dates = [1, 2, 3].map(n => D.addDays(f.date, n));
    wx.showActionSheet({
      itemList: dates.map(d => D.md(d) + ' ' + D.weekLabel(d) + ' 同航班 ' + f.no + ' ' + f.depText),
      success: (r) => {
        const fee = order.changeQuote(this.o);
        wx.showModal({
          title: '确认改期', content: '改到 ' + D.md(dates[r.tapIndex]) + '，改期手续费 ¥' + fee + (fee ? '，将从原支付方式扣款。' : '（本运价免费改期）。'),
          success: (m) => { if (m.confirm) { order.change(this.o.id, dates[r.tapIndex]); this.refresh(); wx.showToast({ title: '改期成功', icon: 'success' }); } }
        });
      }
    });
  },
  copyId() { wx.setClipboardData({ data: this.o.id }); },
  copyTrip() { wx.setClipboardData({ data: order.itinerary(this.o), success: () => wx.showToast({ title: '行程已复制', icon: 'none' }) }); },
  addCal() { cal.addToCalendar(this.o); },
  remove() {
    wx.showModal({
      title: '删除订单', content: '删除后不可恢复，确定删除吗？',
      success: (r) => { if (r.confirm) { store.removeOrder(this.o.id); wx.switchTab({ url: '/pages/orders/orders' }); } }
    });
  },
  rebook() { wx.switchTab({ url: '/pages/index/index' }); }
});
