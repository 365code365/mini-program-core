const app = getApp();

Page({
  data: {
    currentTab: 'all',
    tabs: [
      { key: 'all', name: '全部' },
      { key: 'pending', name: '待付款' },
      { key: 'paid', name: '待发货' },
      { key: 'shipped', name: '待收货' },
      { key: 'completed', name: '已完成' }
    ],
    orders: []
  },

  onLoad: function(options) {
    const status = options.status || 'all';
    this.setData({ currentTab: status });
    this.loadOrders();
  },

  onShow: function() {
    this.loadOrders();
  },

  // 切换标签
  onTabChange: function(e) {
    const tab = e.currentTarget.dataset.tab;
    this.setData({ currentTab: tab });
    this.loadOrders();
  },

  // 加载订单
  loadOrders: function() {
    let orders = app.globalData.orders.slice();

    // 筛选订单
    if (this.data.currentTab !== 'all') {
      orders = orders.filter(order => order.status === this.data.currentTab);
    }

    this.setData({ orders });
  },

  // 取消订单
  onCancelOrder: function(e) {
    const orderId = e.currentTarget.dataset.id;
    wx.showModal({
      title: '确认取消',
      content: '确定要取消该订单吗？',
      success: (res) => {
        if (res.confirm) {
          const orders = app.globalData.orders;
          const index = orders.findIndex(o => o.id === orderId);
          if (index !== -1) {
            orders.splice(index, 1);
            app.saveToStorage();
            this.loadOrders();
            wx.showToast({ title: '订单已取消', icon: 'success' });
          }
        }
      }
    });
  },

  // 去支付
  onPayOrder: function(e) {
    const orderId = e.currentTarget.dataset.id;
    wx.showLoading({ title: '支付中...' });

    setTimeout(() => {
      wx.hideLoading();
      const orders = app.globalData.orders;
      const order = orders.find(o => o.id === orderId);
      if (order) {
        order.status = 'paid';
        order.statusText = '待发货';
        app.saveToStorage();
        this.loadOrders();
        wx.showToast({ title: '支付成功', icon: 'success' });
      }
    }, 1500);
  },

  // 查看物流
  onViewLogistics: function(e) {
    wx.showModal({
      title: '物流信息',
      content: '快递公司: 中通快递\n快递单号: ZT1234567890\n当前状态: 运输中',
      showCancel: false
    });
  },

  // 确认收货
  onConfirmReceipt: function(e) {
    const orderId = e.currentTarget.dataset.id;
    wx.showModal({
      title: '确认收货',
      content: '确认已收到商品吗？',
      success: (res) => {
        if (res.confirm) {
          const orders = app.globalData.orders;
          const order = orders.find(o => o.id === orderId);
          if (order) {
            order.status = 'completed';
            order.statusText = '已完成';
            app.saveToStorage();
            this.loadOrders();
            wx.showToast({ title: '收货成功', icon: 'success' });
          }
        }
      }
    });
  },

  // 再次购买 / 去逛逛
  onBuyAgain: function() {
    wx.switchTab({ url: '/pages/index/index' });
  },

  // 删除订单
  onDeleteOrder: function(e) {
    const orderId = e.currentTarget.dataset.id;
    wx.showModal({
      title: '确认删除',
      content: '确定要删除该订单吗？',
      success: (res) => {
        if (res.confirm) {
          const orders = app.globalData.orders;
          const index = orders.findIndex(o => o.id === orderId);
          if (index !== -1) {
            orders.splice(index, 1);
            app.saveToStorage();
            this.loadOrders();
            wx.showToast({ title: '删除成功', icon: 'success' });
          }
        }
      }
    });
  }
});
