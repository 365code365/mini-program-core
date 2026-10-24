const app = getApp();

Page({
  data: {
    userInfo: {},
    orderCounts: {
      pending: 0,
      paid: 0,
      shipped: 0,
      completed: 0
    }
  },

  onLoad: function(options) {
    this.setData({ userInfo: app.globalData.userInfo });
    this.loadOrderCounts();
  },

  onShow: function() {
    this.loadOrderCounts();
    if (typeof this.getTabBar === 'function' && this.getTabBar()) {
      this.getTabBar().updateCartCount();
    }
  },

  // 加载订单数量
  loadOrderCounts: function() {
    const orders = app.globalData.orders;
    const counts = {
      pending: 0,
      paid: 0,
      shipped: 0,
      completed: 0
    };

    orders.forEach(order => {
      if (order.status === 'pending') counts.pending++;
      else if (order.status === 'paid') counts.paid++;
      else if (order.status === 'shipped') counts.shipped++;
      else if (order.status === 'completed') counts.completed++;
    });

    this.setData({ orderCounts: counts });
  },

  // 查看全部订单
  onViewAllOrders: function() {
    wx.navigateTo({ url: '/pages/orders/orders' });
  },

  // 订单状态点击
  onOrderStatusTap: function(e) {
    const status = e.currentTarget.dataset.status;
    wx.navigateTo({
      url: `/pages/orders/orders?status=${status}`
    });
  },

  // 收货地址
  onAddressManage: function() {
    wx.navigateTo({ url: '/pages/address/address' });
  },

  // 我的优惠券
  onMyCoupons: function() {
    wx.showToast({ title: '暂无优惠券', icon: 'none' });
  },

  // 我的收藏
  onMyFavorites: function() {
    wx.showToast({ title: '暂无收藏', icon: 'none' });
  },

  // 浏览记录
  onHistory: function() {
    wx.showToast({ title: '暂无浏览记录', icon: 'none' });
  },

  // 联系客服
  onContactService: function() {
    wx.showModal({
      title: '联系客服',
      content: '客服电话: 400-888-8888\n工作时间: 9:00-18:00',
      showCancel: false
    });
  },

  // 意见反馈
  onFeedback: function() {
    wx.showToast({ title: '感谢您的反馈', icon: 'none' });
  },

  // 关于我们
  onAbout: function() {
    wx.showModal({
      title: '关于我们',
      content: '精选商城 - 您的品质生活伙伴\n\n版本: v1.0.0',
      showCancel: false
    });
  }
});
