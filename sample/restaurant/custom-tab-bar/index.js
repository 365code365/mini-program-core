Component({
  data: {
    selected: 0,
    color: '#a0a0a8',
    selectedColor: '#ff6a2b',
    list: [
      { pagePath: 'pages/scan/scan', text: '首页', iconPath: '/images/tab/scan-off.png', selectedIconPath: '/images/tab/scan-on.png' },
      { pagePath: 'pages/menu/menu', text: '点餐', iconPath: '/images/tab/menu-off.png', selectedIconPath: '/images/tab/menu-on.png' },
      { pagePath: 'pages/orders/orders', text: '订单', iconPath: '/images/tab/orders-off.png', selectedIconPath: '/images/tab/orders-on.png' },
      { pagePath: 'pages/mine/mine', text: '我的', iconPath: '/images/tab/mine-off.png', selectedIconPath: '/images/tab/mine-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const path = e.currentTarget.dataset.path;
      this.setData({ selected: Number(e.currentTarget.dataset.index) });
      wx.switchTab({ url: '/' + path });
    }
  }
});
