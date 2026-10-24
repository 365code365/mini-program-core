Component({
  data: {
    selected: 0,
    color: '#8a8f99',
    selectedColor: '#1677ff',
    list: [
      { pagePath: '/pages/index/index', text: '首页', iconPath: '/images/tab/home-off.png', selectedIconPath: '/images/tab/home-on.png' },
      { pagePath: '/pages/orders/orders', text: '订单', iconPath: '/images/tab/order-off.png', selectedIconPath: '/images/tab/order-on.png' },
      { pagePath: '/pages/mine/mine', text: '我的', iconPath: '/images/tab/mine-off.png', selectedIconPath: '/images/tab/mine-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const path = e.currentTarget.dataset.path;
      this.setData({ selected: Number(e.currentTarget.dataset.index) });
      wx.switchTab({ url: path });
    }
  }
});
