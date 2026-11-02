// 确认订单：手机号、优惠券（按票价门槛筛）、卖品、支付方式（微信 / 会员卡余额），15 分钟锁座倒计时
const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
function pad(n) { return (n < 10 ? '0' : '') + n; }
Page({
  data: { couponNames: [], coupon: 0, snack: false, phone: '13862216621', left: '15:00', pick: { seats: [] }, payBy: 'wx', balance: 0, vip: false, snackPrice: D.SNACK_PRICE },
  onLoad() {
    const pick = wx.getStorageSync('cinema_pick') || { mid: 'm3', cid: 'c1', day: '今天', time: '19:10', end: '21:45', lang: '国语IMAX', hall: 'IMAX厅', price: 69, seats: ['5排7座', '5排8座'] };
    if (!pick.seats) pick.seats = ['5排7座', '5排8座'];
    if (!pick.ts) pick.ts = Date.now() + 3 * 3600000;
    const coupons = store.availableCoupons(pick.price * pick.seats.length);
    this.coupons = coupons;
    // 默认挑减得最多的那张
    let best = 0;
    coupons.forEach((c, i) => { if (c.off > coupons[best].off) best = i; });
    this.setData({
      pick, m: D.getMovie(pick.mid), c: D.getCinema(pick.cid),
      couponNames: ['不使用优惠券'].concat(coupons.map(c => c.name + ' 减' + c.off + '元')),
      coupon: coupons.length ? best + 1 : 0,
      balance: store.card().balance, vip: store.isVip()
    });
    this.calc();
    this.remain = 900;
    this.timer = setInterval(() => {
      this.remain--;
      if (this.remain <= 0) {
        clearInterval(this.timer);
        wx.showModal({ title: '支付超时', content: '座位已释放，请重新选座', showCancel: false, success: () => wx.navigateBack() });
        return;
      }
      this.setData({ left: pad(Math.floor(this.remain / 60)) + ':' + pad(this.remain % 60) });
    }, 1000);
  },
  onUnload() { clearInterval(this.timer); },
  calc() {
    const c = this.data.coupon > 0 ? this.coupons[this.data.coupon - 1] : null;
    this.q = store.quote(this.data.pick, c ? c.id : '', this.data.snack);
    const q = this.q;
    this.setData({ ticketFee: q.ticketFee, service: q.service, discount: q.discount, total: q.total });
  },
  onPhone(e) { this.setData({ phone: e.detail.value }); },
  onCoupon(e) { this.setData({ coupon: Number(e.detail.value) }); this.calc(); },
  onSnack(e) { this.setData({ snack: e.detail.value }); this.calc(); },
  onPayBy(e) { this.setData({ payBy: e.currentTarget.dataset.way }); },
  onCard() { wx.navigateTo({ url: '/pages/card/card' }); },
  // 从会员卡页充值 / 开通回来：余额与服务费跟着变
  onShow() {
    if (!this.q) return;
    this.setData({ balance: store.card().balance, vip: store.isVip() });
    this.calc();
  },
  finish() {
    clearInterval(this.timer);
    const t = store.placeOrder(this.data.pick, this.q, { payBy: this.data.payBy, phone: this.data.phone });
    wx.showToast({ title: '出票成功', icon: 'success' });
    setTimeout(() => wx.redirectTo({ url: '/pages/order/order?id=' + t.id }), 600);
  },
  onPay() {
    if (!/^1\d{10}$/.test(this.data.phone)) { wx.showToast({ title: '请输入正确的手机号', icon: 'none' }); return; }
    const total = this.data.total;
    const desc = this.data.m.title + ' ' + this.data.pick.time + ' ' + this.data.pick.seats.join(' ');
    if (this.data.payBy === 'card') {
      if (this.data.balance < total) { wx.showToast({ title: '会员卡余额不足，请先充值', icon: 'none' }); return; }
      wx.showModal({
        title: '会员卡支付 ¥' + total, content: desc + '\n支付后余额 ¥' + Math.round((this.data.balance - total) * 100) / 100,
        confirmText: '支付', confirmColor: '#E8243F',
        success: r => r.confirm && this.finish()
      });
      return;
    }
    host.pay(total, desc).then(ok => ok && this.finish());
  }
});
