// 个人中心页面
Page({
  data: {
    loggedIn: false,
    userName: '点击登录',
    couponText: '5张可用',
    userInfo: {
      name: '用户12345',
      level: '普通会员'
    },
    orderStats: {
      pending: 2,
      shipping: 1,
      receiving: 3,
      review: 5
    },
    menuList: [
      { id: 1, name: '我的订单', icon: 'success', color: '#FF6B35', value: '查看全部' },
      { id: 2, name: '收货地址', icon: 'info', color: '#4A90D9', value: '3个地址' },
      { id: 3, name: '我的收藏', icon: 'warn', color: '#FF69B4', value: '12件' },
      { id: 4, name: '优惠券', icon: 'waiting', color: '#52C41A', value: '5张可用' },
      { id: 5, name: '积分商城', icon: 'success', color: '#FFB800', value: '1280积分' },
      { id: 6, name: '帮助中心', icon: 'info', color: '#999', value: '' },
      { id: 7, name: '关于我们', icon: 'info', color: '#999', value: '' }
    ]
  },

  onLoad: function() {
    this.refreshUser();
  },
  onShow: function() {
    this.refreshUser();
  },
  refreshUser: function() {
    var user = wx.getStorageSync('user');
    if (user && user.nickname) {
      this.setData({ loggedIn: true, userName: user.nickname });
    } else {
      this.setData({ loggedIn: false, userName: '点击登录' });
    }
  },

  onUserTap: function() {
    if (!this.data.loggedIn) {
      wx.navigateTo({ url: '/pages/login/login' });
    }
  },

  onOrderTap: function(e) {
    var type = e.currentTarget.dataset.type;
    wx.showToast({ title: '查看' + type + '订单', icon: 'none' });
  },

  onMenuTap: function(e) {
    var id = e.currentTarget.dataset.id;
    if (id === 1) { wx.navigateTo({ url: '/pages/address/address' }); return; }
    if (id === 3) { wx.navigateTo({ url: '/pages/coupon/coupon' }); return; }
    wx.showToast({ title: '功能开发中', icon: 'none' });
  },

  onSettings: function() {
    console.log('⚙️ 设置');
    wx.showToast({ title: '设置', icon: 'none' });
  },

  onLogout: function() {
    var self = this;
    wx.showModal({
      title: '提示',
      content: '确定要退出登录吗？',
      success: function(res) {
        if (res.confirm) {
          wx.removeStorageSync('user');
          self.refreshUser();
          wx.showToast({ title: '已退出', icon: 'none' });
        }
      }
    });
  }
});
