// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，用图片图标自己画
Component({
  data: {
    selected: 0,
    color: '#9aa0ab',
    selectedColor: '#14a06f',
    list: [
      { pagePath: '/pages/index/index', text: '缴费', icon: '/images/tab/home.png', iconOn: '/images/tab/home-on.png' },
      { pagePath: '/pages/records/records', text: '记录', icon: '/images/tab/bill.png', iconOn: '/images/tab/bill-on.png' },
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
