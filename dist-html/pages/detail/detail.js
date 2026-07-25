// 商品详情页：轮播图 / 规格弹层 / 加购 / 立即购买 → 确认订单
var IMAGES = [
  '/assets/p_earbuds.jpg', '/assets/p_watch.jpg', '/assets/p_sneakers.jpg',
  '/assets/p_backpack.jpg', '/assets/p_coffee.jpg', '/assets/p_phone.jpg',
  '/assets/p_phone2.jpg', '/assets/p_cake.jpg', '/assets/p_lipstick.jpg', '/assets/p_thermos.jpg'
];

var CATALOG = {
  101: { name: '无线蓝牙耳机 Pro', desc: '高清音质 主动降噪 40小时超长续航', price: 199, originalPrice: 299, discount: '6.7折', sales: 2341, stock: 999, image: '/assets/p_earbuds.jpg' },
  102: { name: '智能运动手表', desc: '心率监测 GPS定位 7天续航', price: 599, originalPrice: 799, discount: '7.5折', sales: 1856, stock: 500, image: '/assets/p_watch.jpg' },
  103: { name: '潮流运动鞋', desc: '轻量缓震 透气网面 街头百搭', price: 329, originalPrice: 499, discount: '6.6折', sales: 3120, stock: 320, image: '/assets/p_sneakers.jpg' },
  104: { name: '时尚双肩包', desc: '大容量 防泼水 通勤旅行两用', price: 259, originalPrice: 399, discount: '6.5折', sales: 1580, stock: 640, image: '/assets/p_backpack.jpg' },
  105: { name: '精品咖啡礼盒', desc: '中深烘焙 12 包装 手冲意式皆宜', price: 89, originalPrice: 139, discount: '6.4折', sales: 5621, stock: 2000, image: '/assets/p_coffee.jpg' },
  106: { name: '旗舰智能手机', desc: '骁龙旗舰芯 5000mAh 100W 快充', price: 3999, originalPrice: 4699, discount: '8.5折', sales: 890, stock: 210, image: '/assets/p_phone.jpg' }
};

Page({
  data: {
    product: null,
    gallery: [],
    detailNodes: [],
    specVisible: false,
    specs: [
      { id: 1, name: '黑色', selected: true },
      { id: 2, name: '白色', selected: false },
      { id: 3, name: '蓝色', selected: false }
    ],
    selectedSpec: '黑色',
    quantity: 1,
    cartCount: 0,
    reviews: [
      { id: 1, name: '张**', stars: [1, 2, 3, 4, 5], text: '做工扎实，音质比预期好，续航也够用，日常通勤很合适。' },
      { id: 2, name: '李**', stars: [1, 2, 3, 4], text: '包装完好，发货快。第二次回购了，送朋友也合适。' },
      { id: 3, name: '王**', stars: [1, 2, 3, 4, 5], text: '客服响应很快，遇到问题当天就解决了，好评。' }
    ]
  },

  onLoad: function (options) {
    var id = parseInt(options.id) || 101;
    var product = this.buildProduct(id);
    var self = this;
    // 详情用 rich-text 渲染富文本节点
    var nodes = [
      { name: 'p', attrs: { style: 'font-size:13px;color:#666;line-height:1.9' },
        children: [{ type: 'text', text: '【产品特点】高品质材料，经久耐用；精心设计，使用便捷。' }] },
      { name: 'p', attrs: { style: 'font-size:13px;color:#666;line-height:1.9' },
        children: [{ type: 'text', text: '【包装清单】产品 x1 · 说明书 x1 · 保修卡 x1' }] },
      { name: 'p', attrs: { style: 'font-size:13px;color:#FF6B35;line-height:1.9' },
        children: [{ type: 'text', text: '【售后服务】7 天无理由退换，全国联保一年。' }] }
    ];
    this.setData({
      product: product,
      gallery: [product.image, IMAGES[(id + 1) % IMAGES.length], IMAGES[(id + 3) % IMAGES.length]],
      detailNodes: nodes
    });
    this.updateCartCount();
    console.log('📦 商品详情：' + product.name);
    var _ = self;
  },

  onShow: function () {
    this.updateCartCount();
  },

  buildProduct: function (id) {
    var base = CATALOG[id];
    if (!base) {
      base = {
        name: '精选好物 ' + id,
        desc: '优质商品 品质保证 顺丰包邮',
        price: 99 + ((id * 137) % 1900),
        originalPrice: 199 + ((id * 211) % 2600),
        discount: '7折',
        sales: 100 + ((id * 311) % 4800),
        stock: 100 + ((id * 97) % 900),
        image: IMAGES[id % IMAGES.length]
      };
    }
    return {
      id: id, name: base.name, desc: base.desc, price: base.price,
      originalPrice: base.originalPrice, discount: base.discount,
      sales: base.sales, stock: base.stock, image: base.image
    };
  },

  updateCartCount: function () {
    var app = getApp();
    app.getCart();
    this.setData({ cartCount: app.updateCartCount() });
  },

  // ==================== 规格 ====================

  onOpenSpec: function () { this.setData({ specVisible: true }); },
  onCloseSpec: function () { this.setData({ specVisible: false }); },

  onSelectSpec: function (e) {
    var index = e.currentTarget.dataset.index;
    var specs = this.data.specs.map(function (spec, i) {
      spec.selected = (i === index);
      return spec;
    });
    this.setData({ specs: specs, selectedSpec: specs[index].name });
  },

  onIncrease: function () {
    if (this.data.quantity < this.data.product.stock) {
      this.setData({ quantity: this.data.quantity + 1 });
    }
  },

  onDecrease: function () {
    if (this.data.quantity > 1) {
      this.setData({ quantity: this.data.quantity - 1 });
    }
  },

  // ==================== 加购 / 下单 ====================

  onAddCart: function () {
    var self = this;
    var p = this.data.product;
    var item = {
      id: p.id, name: p.name, price: p.price, image: p.image, spec: this.data.selectedSpec
    };
    getApp().addToCart(item, this.data.quantity);
    self.updateCartCount();
    self.setData({ specVisible: false });
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },

  onBuyNow: function () {
    var p = this.data.product;
    var qty = this.data.quantity;
    wx.setStorageSync('checkoutItems', [{
      id: p.id, name: p.name, price: p.price, image: p.image,
      spec: this.data.selectedSpec, quantity: qty
    }]);
    wx.navigateTo({ url: '/pages/order/order?from=detail' });
  },

  onGoCart: function () {
    wx.switchTab({ url: '/pages/cart/cart' });
  },

  onGoHome: function () {
    wx.switchTab({ url: '/pages/index/index' });
  },

  onBack: function () {
    wx.navigateBack();
  }
});
