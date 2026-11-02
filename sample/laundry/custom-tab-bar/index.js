// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画；价目表图标上的角标是购物篮件数。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    cartN: 0,
    color: '#9aa0a8',
    selectedColor: '#1e8cf0',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/price/price', path: 'pages/price/price', text: '价目表', icon: '/images/tab/tag.png', iconOn: '/images/tab/tag-on.png' },
      { pagePath: '/pages/orders/orders', path: 'pages/orders/orders', text: '订单', icon: '/images/tab/list.png', iconOn: '/images/tab/list-on.png' },
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
