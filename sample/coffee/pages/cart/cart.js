const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

function pickTimes() {
  const now = Date.now();
  const list = ['立即制作（约 5 分钟）'];
  const start = Math.ceil((now + 20 * 60000) / 900000) * 900000;
  for (let i = 0; i < 6; i++) list.push('预约 ' + store.hm(start + i * 900000));
  return list;
}

Page({
  data: {
    mode: 'pickup',
    store: data.stores[0],
    pickTimes: [],
    pickIdx: 0,
    list: [],
    coupon: null,
    couponCut: 0,
    couponHint: '',
    deliveryFee: 0,
    usePoints: false,
    usePointsNum: 0,
    pointsCut: 0,
    remark: '',
    pay: 0,
    saved: 0,
    earn: 0
  },

  onLoad() {
    this.setData({ pickTimes: pickTimes() });
  },

  onShow() {
    const storeId = getApp().globalData.storeId;
    // 优惠券页「去使用」会把选中的券写在这里
    const picked = wx.getStorageSync('cf_pick_coupon');
    if (picked) {
      wx.removeStorageSync('cf_pick_coupon');
      this.userCoupon = picked;
    }
    this.setData({
      mode: store.getMode(),
      store: data.stores.find(s => s.id === storeId) || data.stores[0]
    });
    this.calc();
  },

  validCoupons() {
    const used = wx.getStorageSync('cf_used_coupons') || [];
    const extra = wx.getStorageSync('cf_extra_coupons') || [];
    return data.coupons.filter(c => c.status === 'valid' && used.indexOf(c.id) < 0).concat(extra);
  },

  // 一张券能省多少（不满足条件返回 0）
  cutOf(c, s) {
    if (!c || s.amount < c.min) return 0;
    if (c.rate) return this.data.mode === 'delivery' ? store.round2(s.amount * (1 - c.rate)) : 0;
    if (c.fixed) {
      const cheapest = s.list.filter(x => x.size === '中杯' && data.findDrink(x.drinkId).cat !== 'new').map(x => x.unit).sort((a, b) => a - b)[0];
      return cheapest ? store.round2(Math.max(0, cheapest - c.amount)) : 0;
    }
    return Math.min(c.amount, s.amount);
  },

  calc() {
    const s = store.summary();
    const coupons = this.validCoupons();
    let coupon = null;
    if (this.userCoupon !== undefined) {
      coupon = coupons.find(c => c.id === this.userCoupon) || null;
    } else {
      // 默认挑最省的一张
      coupons.forEach(c => { if (this.cutOf(c, s) > this.cutOf(coupon, s)) coupon = c; });
    }
    const couponCut = this.cutOf(coupon, s);
    const usable = coupons.filter(c => this.cutOf(c, s) > 0).length;
    const deliveryFee = this.data.mode === 'delivery' && s.amount < 29 ? 6 : 0;
    const points = wx.getStorageSync('cf_points') || 2380;
    const usePointsNum = Math.min(500, Math.floor(points / 100) * 100);
    const pointsCut = usePointsNum / 100;
    let pay = s.amount - couponCut + deliveryFee - (this.data.usePoints ? pointsCut : 0);
    pay = Math.max(0.01, store.round2(pay));
    this.setData({
      list: s.list,
      coupon,
      couponCut,
      couponHint: usable ? `${usable} 张可用` : '暂无可用',
      deliveryFee,
      usePointsNum,
      pointsCut,
      pay,
      saved: store.round2(couponCut + (this.data.usePoints ? pointsCut : 0)),
      earn: Math.floor(pay)
    });
  },

  onMode(e) {
    store.setMode(e.currentTarget.dataset.mode);
    this.setData({ mode: e.currentTarget.dataset.mode });
    this.calc();
  },

  onPickTime(e) {
    this.setData({ pickIdx: Number(e.detail.value) });
  },

  onMinus(e) {
    store.changeQty(e.currentTarget.dataset.key, -1);
    this.calc();
  },

  onPlus(e) {
    store.changeQty(e.currentTarget.dataset.key, 1);
    this.calc();
  },

  onClear() {
    wx.showModal({
      title: '清空购物车',
      content: '确定清空已选的饮品吗？',
      success: (res) => {
        if (res.confirm) {
          store.saveCart([]);
          this.calc();
        }
      }
    });
  },

  onCoupon() {
    const s = store.summary();
    const coupons = this.validCoupons();
    const items = coupons.map(c => {
      const cut = this.cutOf(c, s);
      return cut ? `${c.title}  省 ¥${cut}` : `${c.title}（${c.cond}）`;
    });
    items.push('不使用优惠券');
    wx.showActionSheet({
      itemList: items,
      success: (res) => {
        if (res.tapIndex === coupons.length) {
          this.userCoupon = '';
        } else {
          const c = coupons[res.tapIndex];
          if (!this.cutOf(c, s)) {
            wx.showToast({ title: '当前订单不满足使用条件', icon: 'none' });
            return;
          }
          this.userCoupon = c.id;
        }
        this.calc();
      }
    });
  },

  onPoints(e) {
    this.setData({ usePoints: e.detail.value });
    this.calc();
  },

  onRemark(e) {
    this.setData({ remark: e.detail.value });
  },

  onGoMenu() {
    wx.switchTab({ url: '/pages/menu/menu' });
  },

  onPay() {
    const s = store.summary();
    wx.showModal({
      title: '确认支付',
      content: `微信支付 ¥${this.data.pay}`,
      confirmText: '支付',
      success: (res) => {
        if (!res.confirm) return;
        const now = Date.now();
        const seq = (wx.getStorageSync('cf_seq') || 216) + 1;
        wx.setStorageSync('cf_seq', seq);
        const order = {
          id: 'SG' + now.toString().slice(-10),
          code: (this.data.mode === 'pickup' ? 'A' : 'W') + seq,
          mode: this.data.mode,
          storeId: this.data.store.id,
          createdAt: now,
          items: s.list.map(x => ({ name: x.name, spec: `${x.size}/${x.temp}/${x.sugar}`, qty: x.qty, total: x.total, img: x.img })),
          count: s.count,
          pay: this.data.pay,
          remark: this.data.remark,
          pickTime: this.data.mode === 'pickup' ? this.data.pickTimes[this.data.pickIdx] : '约 30 分钟送达'
        };
        store.addOrder(order);
        store.saveCart([]);
        if (this.data.coupon && !String(this.data.coupon.id).startsWith('x')) {
          const used = wx.getStorageSync('cf_used_coupons') || [];
          used.push(this.data.coupon.id);
          wx.setStorageSync('cf_used_coupons', used);
        }
        const points = (wx.getStorageSync('cf_points') || 2380) - (this.data.usePoints ? this.data.usePointsNum : 0) + this.data.earn;
        wx.setStorageSync('cf_points', points);
        wx.showToast({ title: '支付成功', icon: 'success' });
        wx.switchTab({ url: '/pages/pickup/pickup' });
      }
    });
  }
});
