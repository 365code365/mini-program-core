// 自定义 tabBar：引擎的原生 tabBar 不画 iconPath（只画文字首字），所以用 image 自己画图标
Component({
  data: {
    selected: 0,
    color: '#9aa0a6',
    selectedColor: '#6C5CE7',
    list: [
      { pagePath: '/pages/index/index', text: '广场', iconPath: '/images/tab-home.png', selectedIconPath: '/images/tab-home-on.png' },
      { pagePath: '/pages/create/create', text: '发起', iconPath: '/images/tab-create.png', selectedIconPath: '/images/tab-create-on.png' },
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
