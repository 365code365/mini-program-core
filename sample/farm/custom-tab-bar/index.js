// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都写。
// 购物车角标由购物车 / 详情页调 setData({ badge }) 更新。
Component({
  data: {
    selected: 0,
    badge: 0,
    color: '#9aa0a8',
    selectedColor: '#3f9a3c',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/live/live', path: 'pages/live/live', text: '直播', icon: '/images/tab/live.png', iconOn: '/images/tab/live-on.png' },
      { pagePath: '/pages/cart/cart', path: 'pages/cart/cart', text: '购物车', icon: '/images/tab/cart.png', iconOn: '/images/tab/cart-on.png' },
      { pagePath: '/pages/mine/mine', path: 'pages/mine/mine', text: '我的', icon: '/images/tab/user.png', iconOn: '/images/tab/user-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const i = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[i].pagePath });
    }
  }
});
