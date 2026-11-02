const store = require('../../utils/store.js');
const orders = require('../../utils/orders.js');
const host = require('../../utils/host.js');

const MIN = store.MIN;
const HOUR = 60 * MIN;

// 配送时段：立即送达 + 今天还来得及的整点档（1 小时后起）+ 明天 9~21 点，最多 10 档
function deliverySlots(now) {
  const slots = [{ label: `立即送达（预计 ${store.hm(now + 30 * MIN)} 前）`, start: 0, end: 0 }];
  const d = new Date(now);
  const day0 = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  for (let k = 0; k < 2 && slots.length < 10; k++) {
    for (let h = 9; h < 22 && slots.length < 10; h++) {
      const start = day0 + k * 24 * HOUR + h * HOUR;
      if (start < now + HOUR) continue;
      slots.push({ label: `${k ? '明天' : '今天'} ${store.pad(h)}:00-${store.pad(h + 1)}:00`, start, end: start + HOUR });
    }
  }
  return slots;
}

Page({
  data: {
    from: 'cart',
    addr: null,
    slotLabels: [],
    slotIdx: 0,
    lines: [],
    count: 0,
    goods: 0,
    fee: 0,
    coupon: null,
    couponCut: 0,
    couponText: '',
    beans: 0,
    useBeans: false,
    beansNum: 0,
    beansCut: 0,
    remark: '',
    payBy: 'wechat',
    balance: 0,
    pay: 0,
    saved: 0
  },

  onLoad() {
    const co = store.getCheckout();
    this.slots = deliverySlots(Date.now());
    this.setData({ from: co ? co.from : 'cart', slotLabels: this.slots.map(s => s.label) });
    this.source = co ? co.lines : [];
  },

  onShow() {
    this.setData({ addr: store.addrView(store.currentAddr()) });
    this.calc();
  },

  // 价格：商品 → 运费（满 39 免）→ 优惠券（商品券或免运费券，一单一张）→ 鲜豆抵扣
  calc() {
    const lines = this.source.map(store.expand);
    const goods = store.round2(lines.reduce((n, x) => n + x.total, 0));
    const fee = store.shipFee(goods);
    const cut = c => store.couponCut(c, lines, fee);
    const coupons = store.validCoupons();
    let coupon = null;
    if (this.userCoupon !== undefined) coupon = coupons.find(c => c.id === this.userCoupon) || null;
    else coupon = store.bestCoupon(lines, fee);
    const couponCut = cut(coupon);
    const usable = coupons.filter(c => cut(c) > 0).length;
    let couponText = usable ? `${usable} 张可用` : '暂无可用';
    if (couponCut) couponText = '-¥' + couponCut;
    else if (this.userCoupon === '') couponText = '不使用优惠券';
    const beforeBeans = store.round2(goods + fee - couponCut);
    const beansNum = store.beansUsable(beforeBeans);
    const useBeans = this.data.useBeans && beansNum > 0;
    const beansCut = useBeans ? beansNum / 100 : 0;
    const pay = Math.max(0.01, store.round2(beforeBeans - beansCut));
    const balance = store.getWallet().balance;
    this.setData({
      lines,
      count: lines.reduce((n, x) => n + x.qty, 0),
      goods,
      fee,
      coupon: couponCut ? coupon : null,
      couponCut,
      couponText,
      beans: store.getBeans(),
      beansNum,
      useBeans,
      beansCut,
      pay,
      balance,
      // 余额不够时自动改回微信支付
      payBy: this.data.payBy === 'wallet' && balance < pay ? 'wechat' : this.data.payBy,
      saved: store.round2(couponCut + beansCut)
    });
  },

  onAddr() {
    wx.navigateTo({ url: '/pages/address/address?select=1' });
  },

  onSlot(e) {
    this.setData({ slotIdx: Number(e.detail.value) });
  },

  onCoupon() {
    const lines = this.data.lines;
    const fee = this.data.fee;
    // 操作菜单最多 6 项：能用的排前面，留一项「不使用」
    const coupons = store.validCoupons()
      .map(c => ({ c, cut: store.couponCut(c, lines, fee) }))
      .sort((a, b) => b.cut - a.cut)
      .slice(0, 5);
    if (!coupons.length) {
      wx.showToast({ title: '暂无可用优惠券', icon: 'none' });
      return;
    }
    const items = coupons.map(x => (x.cut ? `${x.c.title}  省 ¥${x.cut}` : `${x.c.title}（未满足条件）`)).concat(['不使用优惠券']);
    wx.showActionSheet({
      itemList: items,
      success: (res) => {
        if (res.tapIndex === coupons.length) {
          this.userCoupon = '';
        } else {
          const x = coupons[res.tapIndex];
          if (!x.cut) {
            wx.showToast({ title: x.c.kind === 'ship' ? '本单已免运费，无需用券' : `${x.c.cond}，未满 ¥${x.c.min}`, icon: 'none' });
            return;
          }
          this.userCoupon = x.c.id;
        }
        this.calc();
      }
    });
  },

  onBeans(e) {
    this.setData({ useBeans: e.detail.value });
    this.calc();
  },

  onRemark(e) {
    this.setData({ remark: e.detail.value });
  },

  onPayBy(e) {
    const by = e.currentTarget.dataset.by;
    if (by === 'wallet' && this.data.balance < this.data.pay) {
      this.askRecharge();
      return;
    }
    this.setData({ payBy: by });
  },

  askRecharge() {
    wx.showModal({
      title: '余额不足',
      content: `账户余额 ¥${this.data.balance}，本单需支付 ¥${this.data.pay}`,
      confirmText: '去充值',
      success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/wallet/wallet' }); }
    });
  },

  // 提交订单：先落一张待付款的单（券、鲜豆锁住），再去付款；付款取消就留在「待付款」里
  onSubmit() {
    const d = this.data;
    if (this.submitting) return;
    if (!d.lines.length) {
      wx.showToast({ title: '没有要结算的商品', icon: 'none' });
      return;
    }
    if (!d.addr) {
      wx.showToast({ title: '请先添加收货地址', icon: 'none' });
      return;
    }
    if (!d.addr.inRange) {
      wx.showToast({ title: '该地址超出配送范围，请换一个地址', icon: 'none' });
      return;
    }
    if (d.payBy === 'wallet' && d.balance < d.pay) {
      this.askRecharge();
      return;
    }
    const slot = this.slots[d.slotIdx];
    const o = orders.create({
      items: d.lines.map(x => ({ key: x.key, id: x.id, name: x.name, img: x.img, spec: x.spec, qty: x.qty, unit: x.unit, total: x.total, flash: x.flash })),
      count: d.count,
      goods: d.goods,
      fee: d.fee,
      couponId: d.coupon ? d.coupon.id : '',
      couponTitle: d.coupon ? d.coupon.title : '',
      couponKind: d.coupon ? d.coupon.kind : '',
      couponCut: d.couponCut,
      beansUsed: d.useBeans ? d.beansNum : 0,
      beansCut: d.beansCut,
      pay: d.pay,
      payBy: d.payBy,
      remark: d.remark.trim(),
      addr: { name: d.addr.name, phone: d.addr.phoneMask, text: d.addr.text },
      slotLabel: slot.start ? slot.label : '立即送达',
      slotStart: slot.start,
      slotEnd: slot.end
    });
    // 购物车里结算的这几件移掉；立即购买的不动购物车
    if (d.from === 'cart') {
      const keys = d.lines.map(x => x.key);
      store.saveCart(store.getCart().filter(x => keys.indexOf(x.id + '|' + x.spec) < 0));
    }
    store.clearCheckout();
    this.submitting = true;
    this.payOrder(o);
  },

  onBack() {
    wx.switchTab({ url: '/pages/cart/cart' });
  },

  payOrder(o) {
    const done = ok => {
      this.submitting = false;
      if (ok && orders.markPaid(o.id, o.payBy)) {
        wx.showToast({ title: '支付成功', icon: 'success' });
      } else {
        wx.showToast({ title: '订单已保存，可在 15 分钟内支付', icon: 'none' });
      }
      wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + o.id });
    };
    if (o.payBy === 'wallet') {
      wx.showModal({
        title: '余额支付',
        content: `账户余额 ¥${this.data.balance}，本单支付 ¥${o.pay}`,
        confirmText: '确认支付',
        success: r => done(!!r.confirm)
      });
      return;
    }
    host.pay(o.pay, `鲜到家超市 · ${o.count} 件商品`).then(done);
  }
});
