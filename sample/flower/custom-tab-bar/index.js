// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，用图片图标自己画
Component({
  data: {
    selected: 0,
    color: '#9aa0ab',
    selectedColor: '#ef5b7e',
    list: [
      { pagePath: '/pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/cart/cart', text: '购物车', icon: '/images/tab/cart.png', iconOn: '/images/tab/cart-on.png' },
      { pagePath: '/pages/orders/orders', text: '订单', icon: '/images/tab/doc.png', iconOn: '/images/tab/doc-on.png' },
      { pagePath: '/pages/mine/mine', text: '我的', icon: '/images/tab/user.png', iconOn: '/images/tab/user-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const i = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[i].pagePath });
    }
  }
});
