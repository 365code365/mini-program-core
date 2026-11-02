// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画；购物车带数量角标。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    badge: 0,
    color: '#9aa0a8',
    selectedColor: '#12a37f',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/category/category', path: 'pages/category/category', text: '分类', icon: '/images/tab/cat.png', iconOn: '/images/tab/cat-on.png' },
      { pagePath: '/pages/cart/cart', path: 'pages/cart/cart', text: '购物车', icon: '/images/tab/cart.png', iconOn: '/images/tab/cart-on.png' },
      { pagePath: '/pages/mine/mine', path: 'pages/mine/mine', text: '我的', icon: '/images/tab/user.png', iconOn: '/images/tab/user-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const i = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[i].pagePath });
    },
    // 小球落进购物车时角标弹一下（this.animate 关键帧）
    bump(count) {
      this.setData({ badge: count });
      this.animate('.tb-badge', [{ scale: [1, 1] }, { scale: [1.5, 1.5] }, { scale: [1, 1] }], 300, () => this.clearAnimation('.tb-badge'));
    }
  }
});
