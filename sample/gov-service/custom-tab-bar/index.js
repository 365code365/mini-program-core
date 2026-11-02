// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，所以用图片图标自己画。
// 引擎命中自定义 tabBar 时读节点的 data-index + data-path（不带前导 /）直接切页，wxml 两个都要写。
// badge：「我的办件」上的数字角标（有待补正 / 办理中的办件时显示），由各 tab 页 onShow 刷新。
Component({
  data: {
    selected: 0,
    badge: 0,
    color: '#9aa3b0',
    selectedColor: '#1d64d8',
    list: [
      { pagePath: '/pages/index/index', path: 'pages/index/index', text: '首页', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/orders/orders', path: 'pages/orders/orders', text: '我的办件', icon: '/images/tab/doc.png', iconOn: '/images/tab/doc-on.png' },
      { pagePath: '/pages/license/license', path: 'pages/license/license', text: '证照夹', icon: '/images/tab/card.png', iconOn: '/images/tab/card-on.png' },
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
