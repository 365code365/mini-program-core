// 商城小程序
//
// 购物车与订单都落到 wx.storage：
// globalData 只在单个运行时里有效，编译成 H5 后每个页面是独立文档，
// 只靠 globalData 会一跳页就丢购物车。用 storage 后两端行为一致。
App({
  globalData: {
    userInfo: null,
    cart: [],
    cartCount: 0
  },

  onLaunch: function () {
    console.log('🛒 Mini Shop 启动');
    this.globalData.cart = wx.getStorageSync('cart') || [];
    this.globalData.userInfo = wx.getStorageSync('userInfo') || null;
    this.updateCartCount();
  },

  // ==================== 购物车 ====================

  getCart: function () {
    if (!this.globalData.cart || this.globalData.cart.length === 0) {
      this.globalData.cart = wx.getStorageSync('cart') || [];
    }
    return this.globalData.cart;
  },

  saveCart: function (cart) {
    this.globalData.cart = cart || [];
    wx.setStorageSync('cart', this.globalData.cart);
    this.updateCartCount();
  },

  addToCart: function (product, quantity) {
    quantity = quantity || 1;
    if (!product || typeof product !== 'object' || product.id === undefined) {
      console.log('❌ addToCart 参数不合法');
      return 0;
    }
    var cart = this.getCart();
    var found = false;
    for (var i = 0; i < cart.length; i++) {
      if (cart[i].id === product.id) {
        cart[i].quantity += quantity;
        found = true;
        break;
      }
    }
    if (!found) {
      cart.push({
        id: product.id,
        name: product.name,
        price: product.price,
        image: product.image || '/assets/p_earbuds.jpg',
        spec: product.spec || '默认',
        quantity: quantity,
        selected: true
      });
    }
    this.saveCart(cart);
    console.log('🛒 购物车：' + cart.length + ' 种 / ' + this.globalData.cartCount + ' 件');
    return this.globalData.cartCount;
  },

  removeFromCart: function (id) {
    var cart = this.getCart().filter(function (item) { return item.id !== id; });
    this.saveCart(cart);
  },

  clearCart: function () {
    this.saveCart([]);
  },

  updateCartCount: function () {
    var count = 0;
    (this.globalData.cart || []).forEach(function (item) { count += item.quantity; });
    this.globalData.cartCount = count;
    return count;
  },

  getCartTotal: function () {
    var total = 0;
    (this.globalData.cart || []).forEach(function (item) {
      if (item.selected !== false) { total += item.price * item.quantity; }
    });
    return total.toFixed(2);
  },

  // ==================== 订单 ====================

  getOrders: function () {
    return wx.getStorageSync('orders') || [];
  },

  addOrder: function (order) {
    var orders = this.getOrders();
    order.id = 'SO' + Date.now();
    order.createdAt = this.formatTime(new Date());
    order.status = order.status || 'pending';
    orders.unshift(order);
    wx.setStorageSync('orders', orders);
    return order;
  },

  updateOrderStatus: function (id, status) {
    var orders = this.getOrders().map(function (o) {
      if (o.id === id) { o.status = status; }
      return o;
    });
    wx.setStorageSync('orders', orders);
  },

  // ==================== 工具 ====================

  formatTime: function (d) {
    var p = function (n) { return n < 10 ? '0' + n : '' + n; };
    return d.getFullYear() + '-' + p(d.getMonth() + 1) + '-' + p(d.getDate()) +
      ' ' + p(d.getHours()) + ':' + p(d.getMinutes());
  },

  // 搜索历史
  getSearchHistory: function () {
    return wx.getStorageSync('searchHistory') || [];
  },

  pushSearchHistory: function (keyword) {
    if (!keyword) { return []; }
    var list = this.getSearchHistory().filter(function (k) { return k !== keyword; });
    list.unshift(keyword);
    if (list.length > 10) { list = list.slice(0, 10); }
    wx.setStorageSync('searchHistory', list);
    return list;
  },

  clearSearchHistory: function () {
    wx.setStorageSync('searchHistory', []);
    return [];
  }
});
