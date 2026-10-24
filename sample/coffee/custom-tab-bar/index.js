Component({
  data: {
    selected: 0,
    color: '#a39a92',
    selectedColor: '#6b4226',
    list: [
      { pagePath: '/pages/index/index', text: '首页', iconPath: '/images/tab/home-off.png', selectedIconPath: '/images/tab/home-on.png' },
      { pagePath: '/pages/menu/menu', text: '点单', iconPath: '/images/tab/menu-off.png', selectedIconPath: '/images/tab/menu-on.png' },
      { pagePath: '/pages/pickup/pickup', text: '取餐', iconPath: '/images/tab/order-off.png', selectedIconPath: '/images/tab/order-on.png' },
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
