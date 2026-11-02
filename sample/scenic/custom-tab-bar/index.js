// 自定义 tabBar：引擎原生 tabBar 不画 iconPath，用图片图标自己画；「订单」tab 带待使用角标（utils/tab.js 写入）。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    badge: '',
    color: '#a0a6b0',
    selectedColor: '#13a07b',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/guide/guide', path: 'pages/guide/guide', text: '导览', icon: '/images/tab/guide.png', iconOn: '/images/tab/guide-on.png' },
      { pagePath: '/pages/orders/orders', path: 'pages/orders/orders', text: '订单', icon: '/images/tab/order.png', iconOn: '/images/tab/order-on.png' },
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
