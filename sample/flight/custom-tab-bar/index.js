// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，所以用图片图标自己画；「订单」上的角标是待支付订单数。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，所以 wxml 两个都要写
Component({
  data: {
    selected: 0,
    unpaid: 0,
    color: '#9aa3b2',
    selectedColor: '#1f6fff',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '机票', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
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
