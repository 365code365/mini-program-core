const app = getApp();
const COUPONS = [{ name: '不使用优惠券', off: 0 }, { name: '新人立减 10 元', off: 10 }, { name: '周三半价日 减 20 元', off: 20 }, { name: '会员观影券 减 5 元', off: 5 }];
function pad(n) { return (n < 10 ? '0' : '') + n; }
Page({
  data: { couponNames: COUPONS.map(c => c.name), coupon: 1, snack: false, phone: '13862216621', left: '15:00', pick: { seats: [] } },
  onLoad() {
    const pick = wx.getStorageSync('cinema_pick') || { mid: 'm3', cid: 'c1', day: '今天', time: '19:10', end: '21:45', lang: '国语IMAX', hall: 'IMAX厅', price: 69, seats: ['5排7座', '5排8座'] };
    if (!pick.seats) pick.seats = ['5排7座', '5排8座'];
    this.setData({ pick, m: app.getMovie(pick.mid), c: app.getCinema(pick.cid) });
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
    const n = this.data.pick.seats.length;
    const ticketFee = this.data.pick.price * n;
    const service = 3 * n;
    const discount = Math.min(COUPONS[this.data.coupon].off, ticketFee);
    this.setData({ ticketFee, service, discount, total: ticketFee + service + (this.data.snack ? 39 : 0) - discount });
  },
  onPhone(e) { this.setData({ phone: e.detail.value }); },
  onCoupon(e) { this.setData({ coupon: Number(e.detail.value) }); this.calc(); },
  onSnack(e) { this.setData({ snack: e.detail.value }); this.calc(); },
  onPay() {
    if (!/^1\d{10}$/.test(this.data.phone)) { wx.showToast({ title: '请输入正确的手机号', icon: 'none' }); return; }
    wx.showModal({
      title: '确认支付 ¥' + this.data.total, content: this.data.m.title + '\n' + this.data.pick.day + ' ' + this.data.pick.time + ' ' + this.data.pick.seats.join(' '),
      confirmText: '支付', confirmColor: '#E8243F',
      success: (r) => {
        if (!r.confirm) return;
        clearInterval(this.timer);
        const code = String(Math.floor(10000000 + Math.random() * 89999999));
        const t = Object.assign({}, this.data.pick, { id: 'TK' + Date.now(), code, total: this.data.total, snack: this.data.snack, cinema: this.data.c.name, title: this.data.m.title, poster: this.data.m.poster, status: 'unused' });
        const list = app.getTickets(); list.unshift(t); app.saveTickets(list);
        wx.removeStorageSync('cinema_pick');
        wx.showToast({ title: '出票成功', icon: 'success' });
        setTimeout(() => wx.switchTab({ url: '/pages/tickets/tickets' }), 800);
      }
    });
  }
});
