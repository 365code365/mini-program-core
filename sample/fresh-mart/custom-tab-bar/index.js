Component({
  data: {
    selected: 0,
    color: '#8a8f99',
    selectedColor: '#12b76a',
    list: [
      { pagePath: '/pages/index/index', text: '首页', iconPath: '/images/tab/home-off.png', selectedIconPath: '/images/tab/home-on.png' },
      { pagePath: '/pages/category/category', text: '分类', iconPath: '/images/tab/cate-off.png', selectedIconPath: '/images/tab/cate-on.png' },
      { pagePath: '/pages/cart/cart', text: '购物车', iconPath: '/images/tab/cart-off.png', selectedIconPath: '/images/tab/cart-on.png' },
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
