// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath，所以用 image 自己画图标
Component({
  data: {
    selected: 0,
    color: '#9aa0a6',
    selectedColor: '#D7263D',
    list: [
      { pagePath: '/pages/index/index', text: '首页', iconPath: '/images/tab-home.png', selectedIconPath: '/images/tab-home-on.png' },
      { pagePath: '/pages/bills/bills', text: '明细', iconPath: '/images/tab-bills.png', selectedIconPath: '/images/tab-bills-on.png' },
      { pagePath: '/pages/wealth/wealth', text: '理财', iconPath: '/images/tab-wealth.png', selectedIconPath: '/images/tab-wealth-on.png' },
      { pagePath: '/pages/mine/mine', text: '我的', iconPath: '/images/tab-mine.png', selectedIconPath: '/images/tab-mine-on.png' }
    ]
  },
  methods: {
    switchTab(e) {
      const idx = e.currentTarget.dataset.index;
      wx.switchTab({ url: this.data.list[idx].pagePath });
    }
  }
});
