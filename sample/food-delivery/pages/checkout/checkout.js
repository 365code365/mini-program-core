const data = require('../../utils/data.js');
const cart = require('../../utils/cart.js');

// 店铺满减：从「满X减Y」标签里取能用上的最大一档
function bestFullCut(tags, amount) {
  let cut = 0;
  tags.forEach(t => {
    const m = /^满(\d+)减(\d+)$/.exec(t);
    if (m && amount >= Number(m[1])) cut = Math.max(cut, Number(m[2]));
  });
  return cut;
}

function timeOptions() {
  const now = Date.now();
  const list = [`立即送出（约 ${cart.hm(now + 30 * 60000)} 送达）`];
  const start = Math.ceil((now + 45 * 60000) / 900000) * 900000;
  for (let i = 0; i < 6; i++) list.push(`${cart.hm(start + i * 900000)} - ${cart.hm(start + (i + 1) * 900000)}`);
  return list;
}

Page({
  data: {
    shop: {},
    items: [],
    address: data.addresses[0],
    times: [],
    timeIdx: 0,
    remarks: ['口味、偏好等要求', '少辣', '不要香菜', '多放米饭', '放门口，不用敲门', '到了打电话'],
    remarkIdx: 0,
    tableware: ['按餐量提供', '不需要餐具（环保）', '1 份', '2 份', '3 份以上'],
    tablewareIdx: 0,
    protect: true,
    coupon: data.coupons[0],
    couponHint: '',
    packFee: 0,
    fullCut: 0,
    discount: 0,
    pay: 0
  },

  onLoad(options) {
    const c = cart.getCart();
    const shopId = options.shop || c.shopId || 's1';
    const shop = data.findShop(shopId);
    const addrId = getApp().globalData.addressId;
    const address = data.addresses.find(a => a.id === addrId) || data.addresses[0];
    this.setData({ shop, address, times: timeOptions() });
    this.calc();
  },

  calc(couponArg) {
    const shop = this.data.shop;
    const s = cart.summary(shop.id);
    const packFee = s.count * shop.packFee;
    const fullCut = bestFullCut(shop.tags, s.amount);
    const base = s.amount - fullCut;
    let coupon = couponArg || this.data.coupon;
    // 第一次进来自动选最划算的红包
    if (!couponArg && !this.couponTouched) {
      coupon = data.coupons.filter(x => base >= x.min).sort((a, b) => b.amount - a.amount)[0];
    }
    if (base < coupon.min || !s.count) coupon = data.coupons[0];
    const usable = data.coupons.filter(x => x.amount && base >= x.min).length;
    const pay = s.count ? Math.max(0.01, cart.round2(s.amount + packFee + shop.deliveryFee - fullCut - coupon.amount)) : 0;
    this.setData({
      items: s.list,
      packFee,
      fullCut,
      coupon,
      couponHint: usable ? `${usable} 个可用` : '暂无可用',
      discount: cart.round2(fullCut + coupon.amount),
      pay
    });
  },

  onAddress() {
    const list = data.addresses;
    wx.showActionSheet({
      itemList: list.map(a => `${a.tag} · ${a.detail}`),
      success: (res) => {
        const address = list[res.tapIndex];
        getApp().globalData.addressId = address.id;
        wx.setStorageSync('fd_address', address.id);
        this.setData({ address });
      }
    });
  },

  onTime(e) {
    this.setData({ timeIdx: Number(e.detail.value) });
  },

  onRemark(e) {
    this.setData({ remarkIdx: Number(e.detail.value) });
  },

  onTableware(e) {
    this.setData({ tablewareIdx: Number(e.detail.value) });
  },

  onProtect(e) {
    this.setData({ protect: e.detail.value });
  },

  onCoupon() {
    const list = data.coupons;
    const base = cart.summary(this.data.shop.id).amount - this.data.fullCut;
    wx.showActionSheet({
      itemList: list.map(c => (c.amount && base < c.min ? `${c.title}（未满足）` : c.title)),
      success: (res) => {
        const c = list[res.tapIndex];
        if (base < c.min) {
          wx.showToast({ title: `还差 ¥${cart.round2(c.min - base)} 可用`, icon: 'none' });
          return;
        }
        this.couponTouched = true;
        this.calc(c);
      }
    });
  },

  onBack() {
    if (getCurrentPages().length > 1) wx.navigateBack();
    else wx.redirectTo({ url: `/pages/shop/shop?id=${this.data.shop.id}` });
  },

  onSubmit() {
    if (!this.data.items.length) {
      wx.showToast({ title: '购物车是空的', icon: 'none' });
      return;
    }
    wx.showLoading({ title: '正在支付' });
    const now = Date.now();
    const order = {
      id: 'BL' + now.toString().slice(-10),
      shopId: this.data.shop.id,
      status: 'delivering',
      createdAt: now,
      items: this.data.items.map(i => ({ name: i.name, qty: i.qty, price: i.price })),
      pay: this.data.pay,
      remark: this.data.remarkIdx ? this.data.remarks[this.data.remarkIdx] : '无',
      tableware: this.data.tableware[this.data.tablewareIdx],
      address: this.data.address,
      eta: this.data.timeIdx ? this.data.times[this.data.timeIdx] : cart.hm(now + 30 * 60000)
    };
    setTimeout(() => {
      wx.hideLoading();
      cart.addOrder(order);
      cart.clear();
      wx.showToast({ title: '支付成功', icon: 'success' });
      wx.redirectTo({ url: `/pages/order-detail/order-detail?id=${order.id}` });
    }, 600);
  }
});
