// 确认订单：地址 / 配送 / 优惠券 / 支付方式 / 提交下单
Page({
  data: {
    items: [],
    address: null,
    shipMethods: ['普通快递（免运费）', '顺丰特快（+12元）', '同城闪送（+18元）'],
    shipIndex: 0,
    shipFee: '0.00',
    coupons: [
      { id: 1, name: '新人无门槛券', min: 0, amount: 10 },
      { id: 2, name: '满99减30', min: 99, amount: 30 },
      { id: 3, name: '满199减50', min: 199, amount: 50 }
    ],
    couponIndex: -1,
    couponText: '未使用',
    couponCut: '0.00',
    couponVisible: false,
    payMethods: [
      { key: 'wechat', name: '微信支付', tip: '推荐' },
      { key: 'alipay', name: '支付宝', tip: '' },
      { key: 'card', name: '银行卡', tip: '' }
    ],
    payMethod: 'wechat',
    remark: '',
    goodsTotal: '0.00',
    payTotal: '0.00',
    submitting: false
  },

  onLoad: function () {
    this.setData({ items: wx.getStorageSync('checkoutItems') || [] });
    this.loadAddress();
    this.recalc();
  },

  onShow: function () {
    this.loadAddress();
    this.recalc();
  },

  loadAddress: function () {
    var picked = wx.getStorageSync('selectedAddress');
    if (!picked) {
      picked = { name: '张三', phone: '13800001111', region: '广东省深圳市南山区', detail: '科技园路 1 号', isDefault: true };
    }
    this.setData({ address: picked });
  },

  recalc: function () {
    var goods = 0;
    this.data.items.forEach(function (i) { goods += i.price * i.quantity; });
    var shipFee = [0, 12, 18][this.data.shipIndex] || 0;
    var cut = 0;
    var idx = this.data.couponIndex;
    var text = '未使用';
    if (idx >= 0) {
      var c = this.data.coupons[idx];
      if (c && goods >= c.min) {
        cut = c.amount;
        text = c.name + ' -¥' + c.amount;
      } else if (c) {
        text = '不满足门槛';
      }
    } else {
      text = this.data.coupons.length + ' 张可用';
    }
    var pay = Math.max(0, goods + shipFee - cut);
    this.setData({
      goodsTotal: goods.toFixed(2),
      shipFee: shipFee.toFixed(2),
      couponCut: cut.toFixed(2),
      couponText: text,
      payTotal: pay.toFixed(2)
    });
  },

  onShipChange: function (e) {
    this.setData({ shipIndex: parseInt(e.detail.value) || 0 });
    this.recalc();
  },

  onPickCoupon: function () { this.setData({ couponVisible: true }); },
  onCloseCoupon: function () { this.setData({ couponVisible: false }); },

  onSelectCoupon: function (e) {
    this.setData({ couponIndex: parseInt(e.currentTarget.dataset.index), couponVisible: false });
    this.recalc();
  },

  onPayChange: function (e) {
    this.setData({ payMethod: e.detail.value });
  },

  onRemarkInput: function (e) {
    this.setData({ remark: e.detail.value });
  },

  onPickAddress: function () {
    wx.navigateTo({ url: '/pages/address/address?picker=1' });
  },

  onSubmit: function () {
    var self = this;
    if (this.data.items.length === 0) {
      wx.showToast({ title: '没有可结算的商品', icon: 'none' });
      return;
    }
    this.setData({ submitting: true });
    wx.showLoading({ title: '提交中...' });
    setTimeout(function () {
      wx.hideLoading();
      var app = getApp();
      var order = app.addOrder({
        items: self.data.items,
        total: self.data.payTotal,
        payMethod: self.data.payMethod,
        ship: self.data.shipMethods[self.data.shipIndex],
        remark: self.data.remark,
        address: self.data.address,
        status: 'paid'
      });
      // 已结算的商品从购物车移除
      var picked = {};
      self.data.items.forEach(function (i) { picked[i.id] = true; });
      var rest = app.getCart().filter(function (i) { return !picked[i.id]; });
      app.saveCart(rest);
      wx.removeStorageSync('checkoutItems');
      self.setData({ submitting: false });
      wx.showToast({ title: '下单成功', icon: 'success' });
      setTimeout(function () {
        wx.redirectTo({ url: '/pages/orders/orders?new=' + order.id });
      }, 700);
    }, 700);
  },

  onBack: function () {
    wx.navigateBack();
  }
});
