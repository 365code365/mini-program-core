// 我的：订单统计 / 功能入口 / 登录态
Page({
  data: {
    userName: '未登录',
    loggedIn: false,
    points: 0,
    orderStats: [],
    menus: []
  },

  onShow: function () {
    var app = getApp();
    var user = wx.getStorageSync('userInfo');
    var orders = app.getOrders();
    var count = function (status) {
      return orders.filter(function (o) { return o.status === status; }).length;
    };
    var cartCount = app.updateCartCount();
    this.setData({
      userName: user ? user.name : '未登录',
      loggedIn: !!user,
      points: user ? (user.points || 1280) : 0,
      orderStats: [
        { key: 'pending', name: '待付款', count: count('pending') },
        { key: 'paid', name: '待发货', count: count('paid') },
        { key: 'shipping', name: '待收货', count: count('shipping') },
        { key: 'done', name: '已完成', count: count('done') }
      ],
      menus: [
        { key: 'address', name: '收货地址', icon: 'info_no_circle', color: '#FF6B35', value: '' },
        { key: 'coupon', name: '优惠券', icon: 'arrow_down', color: '#FAAD14', value: wx.getStorageSync('couponClaimed') ? '已领取' : '3 张可领' },
        { key: 'cart', name: '购物车', icon: 'clock', color: '#52C41A', value: cartCount ? cartCount + ' 件' : '空' },
        { key: 'search', name: '搜索历史', icon: 'search', color: '#13c2c2', value: app.getSearchHistory().length + ' 条' },
        { key: 'showcase', name: '能力展示', icon: 'star', color: '#EB2F96', value: '' },
        { key: 'form', name: '表单与事件', icon: 'success_no_circle', color: '#722ed1', value: '' },
        { key: 'canvas', name: 'Canvas 绘图', icon: 'plus', color: '#2F54EB', value: '' },
        { key: 'components', name: '组件示例', icon: 'warn_no_circle', color: '#FA541C', value: '' }
      ]
    });
  },

  onUserTap: function () {
    if (this.data.loggedIn) {
      wx.showToast({ title: '已登录：' + this.data.userName, icon: 'none' });
    } else {
      wx.navigateTo({ url: '/pages/login/login' });
    }
  },

  onAllOrders: function () {
    wx.navigateTo({ url: '/pages/orders/orders' });
  },

  onOrderTap: function (e) {
    wx.navigateTo({ url: '/pages/orders/orders?status=' + e.currentTarget.dataset.type });
  },

  onMenuTap: function (e) {
    var key = e.currentTarget.dataset.key;
    if (key === 'cart') {
      wx.switchTab({ url: '/pages/cart/cart' });
      return;
    }
    var routes = {
      address: '/pages/address/address',
      coupon: '/pages/coupon/coupon',
      search: '/pages/search/search',
      showcase: '/pages/showcase/showcase',
      form: '/pages/form/form',
      canvas: '/pages/canvas/canvas',
      components: '/pages/components/components'
    };
    if (routes[key]) {
      wx.navigateTo({ url: routes[key] });
    }
  },

  onLogout: function () {
    var self = this;
    if (!this.data.loggedIn) {
      wx.navigateTo({ url: '/pages/login/login' });
      return;
    }
    wx.showModal({
      title: '退出登录',
      content: '确定要退出当前账号吗？',
      success: function (res) {
        if (res.confirm) {
          wx.removeStorageSync('userInfo');
          getApp().globalData.userInfo = null;
          self.onShow();
          wx.showToast({ title: '已退出', icon: 'success' });
        }
      }
    });
  }
});
