const app = getApp();
const data = require('../../utils/data.js');

Page({
  data: {
    productId: null,
    product: {},
    cartCount: 0,
    recommends: []
  },

  onLoad: function(options) {
    const productId = parseInt(options.id);
    this.setData({ productId });
    this.loadProductDetail(productId);
  },

  onShow: function() {
    this.setData({ cartCount: app.globalData.cartCount });
  },

  // 加载商品详情（没有带 id 或 id 不认识时展示第一件商品）
  loadProductDetail: function(productId) {
    const product = data.get(productId) || data.products[0];
    const recommends = data.products.filter(p => p.id !== product.id && p.cat === product.cat).concat(
      data.products.filter(p => p.cat !== product.cat)
    ).slice(0, 3);
    this.setData({
      product: { ...product, stock: 999 },
      recommends
    });
  },

  onRecommendTap: function(e) {
    wx.redirectTo({ url: `/pages/detail/detail?id=${e.currentTarget.dataset.id}` });
  },

  // 联系客服
  onContact: function() {
    wx.showModal({
      title: '联系客服',
      content: '客服电话: 400-888-8888\n工作时间: 9:00-18:00',
      showCancel: false
    });
  },

  // 购物车
  onCart: function() {
    wx.switchTab({ url: '/pages/cart/cart' });
  },

  // 加入购物车
  onAddCart: function() {
    app.addToCart(this.data.product, 1);
    this.setData({ cartCount: app.globalData.cartCount });
  },

  // 立即购买
  onBuyNow: function() {
    const product = this.data.product;
    app.globalData.checkoutProducts = [{
      ...product,
      quantity: 1,
      selected: true
    }];
    app.globalData.checkoutTotal = product.price;
    wx.navigateTo({
      url: `/pages/address/address?from=checkout`
    });
  }
});
