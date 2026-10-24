// 精选商城小程序
const data = require('./utils/data.js');
App({
  globalData: {
    userInfo: {
      nickName: '橙子小姐',
      avatar: '/assets/avatar.jpg',
      level: '黄金会员'
    },
    cart: [],
    cartCount: 0,
    orders: [],
    addresses: [
      {
        id: 1,
        name: '张三',
        phone: '138****8888',
        address: '北京市朝阳区建国路88号 SOHO现代城 2号楼1203',
        tag: '家',
        isDefault: true
      },
      {
        id: 2,
        name: '李四',
        phone: '139****6666',
        address: '上海市浦东新区世纪大道100号 环球金融中心 32F',
        tag: '公司',
        isDefault: false
      }
    ]
  },

  onLaunch: function() {
    console.log('🛒 精选商城启动');
    this.loadFromStorage();
  },

  // 从本地存储加载数据
  loadFromStorage: function() {
    try {
      // 本地存储没有数据（首次启动）时放几件演示商品与订单，页面一打开就不是空的
      var cart = wx.getStorageSync('cart');
      this.globalData.cart = cart ? cart : data.demoCart();
      this.updateCartCount();
      var orders = wx.getStorageSync('orders');
      this.globalData.orders = orders ? orders : data.demoOrders();
    } catch (e) {
      console.error('加载本地数据失败', e);
    }
  },

  // 保存数据到本地存储
  saveToStorage: function() {
    try {
      wx.setStorageSync('cart', this.globalData.cart);
      wx.setStorageSync('orders', this.globalData.orders);
    } catch (e) {
      console.error('保存数据失败', e);
    }
  },

  // 添加商品到购物车
  addToCart: function(product, quantity) {
    quantity = quantity || 1;
    var cart = this.globalData.cart;
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
        desc: product.desc,
        price: product.price,
        image: product.image,
        quantity: quantity,
        selected: true
      });
    }

    this.updateCartCount();
    this.saveToStorage();
    wx.showToast({ title: '已加入购物车', icon: 'success' });
  },

  // 更新购物车数量
  updateCartCount: function() {
    var count = 0;
    this.globalData.cart.forEach(function(item) {
      count += item.quantity;
    });
    this.globalData.cartCount = count;
  },

  // 获取购物车总价
  getCartTotal: function() {
    var total = 0;
    this.globalData.cart.forEach(function(item) {
      if (item.selected) {
        total += item.price * item.quantity;
      }
    });
    return total.toFixed(2);
  },

  // 获取选中的商品数量
  getSelectedCount: function() {
    var count = 0;
    this.globalData.cart.forEach(function(item) {
      if (item.selected) {
        count += item.quantity;
      }
    });
    return count;
  },

  // 创建订单
  createOrder: function(address, products, total) {
    var order = {
      id: Date.now(),
      orderNo: 'JX' + Date.now(),
      products: products,
      total: total,
      address: address,
      status: 'pending',
      statusText: '待付款',
      createTime: new Date().toLocaleString()
    };

    this.globalData.orders.unshift(order);
    this.saveToStorage();
    return order;
  }
});
