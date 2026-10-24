// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath（只画文字首字），所以用 image 自己画图标
Component({
  data: {
    selected: 0,
    color: '#9aa0a6',
    selectedColor: '#00B87A',
    list: [
      { pagePath: '/pages/index/index', text: '找桩', iconPath: '/images/tab-home.png', selectedIconPath: '/images/tab-home-on.png' },
      { pagePath: '/pages/orders/orders', text: '订单', iconPath: '/images/tab-orders.png', selectedIconPath: '/images/tab-orders-on.png' },
      { pagePath: '/pages/wallet/wallet', text: '钱包', iconPath: '/images/tab-wallet.png', selectedIconPath: '/images/tab-wallet-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const idx = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[idx].pagePath });
    }
  }
});
