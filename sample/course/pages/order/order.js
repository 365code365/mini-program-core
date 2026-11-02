const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
// 确认订单：限时价 + 优惠券（自动选最省的，可改）→ 微信支付（宿主没有支付能力时页内确认）→ 开通课程
Page({
  data: { c: null, coupons: [], coupon: null, discount: 0, pay: 0, paid: false, order: null },
  onLoad(q) {
    const c = data.courseOf(q.id) || data.courses[0];
    this.c = c;
    this.setData({ c: { id: c.id, title: c.title, cover: c.cover, mark: c.mark, lessons: c.lessons, hours: c.hours, teacher: data.teacherOf(c.teacher).name, price: c.price, origin: c.origin } });
    if (store.owned(c.id)) {
      wx.showToast({ title: '你已拥有这门课', icon: 'none' });
      setTimeout(() => wx.redirectTo({ url: '/pages/lesson/lesson?cid=' + c.id }), 600);
      return;
    }
    const coupons = c.price > 0 ? store.usableCoupons(c.price).sort((a, b) => b.cut - a.cut) : [];
    this.setData({ coupons });
    this.use(coupons[0] || null);
  },
  use(q) {
    const c = this.c;
    this.setData({ coupon: q, discount: c.origin - c.price, pay: Math.max(0, c.price - (q ? q.cut : 0)) });
  },
  pickCoupon() {
    const list = this.data.coupons.slice(0, 5);
    if (!list.length) { wx.showToast({ title: this.c.price ? '没有满足门槛的优惠券' : '免费课程无需用券', icon: 'none' }); return; }
    wx.showActionSheet({ itemList: list.map(q => q.name + ' 减 ¥' + q.cut).concat(['不使用优惠券']), success: r => {
      this.use(r.tapIndex < list.length ? list[r.tapIndex] : null);
    } });
  },
  submit() {
    if (this.paying || this.data.paid) return;
    const c = this.c;
    const done = () => {
      const o = store.placeOrder(c, this.data.coupon);
      this.setData({ paid: true, order: o });
      wx.setNavigationBarTitle({ title: c.price === 0 ? '领取成功' : '支付成功' });
    };
    if (this.data.pay === 0) { done(); return; }
    this.paying = true;
    host.pay(this.data.pay, '知学课堂 ·「' + c.title + '」').then(ok => {
      this.paying = false;
      if (ok) done();
    });
  },
  study() { wx.redirectTo({ url: '/pages/lesson/lesson?cid=' + this.c.id }); },
  orders() { wx.redirectTo({ url: '/pages/orders/orders' }); }
});
